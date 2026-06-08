use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct VerificationPolicy {
    pub verify_issues: bool,
    pub verify_pull_requests: bool,
    pub exempt_collaborators: bool,
    pub exempt_verified_bots: bool,
    pub reverify_after_days: Option<u32>,
    pub check_mode: CheckMode,
    pub apply_label: Option<String>,
    pub verified_label: Option<String>,
    pub pending_label: Option<String>,
    pub comment_on_required: bool,
    pub close_unverified: bool,
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            verify_issues: true,
            verify_pull_requests: true,
            exempt_collaborators: true,
            exempt_verified_bots: true,
            reverify_after_days: Some(90),
            check_mode: CheckMode::Enforce,
            apply_label: Some("human-auth-required".into()),
            verified_label: Some("human-auth-verified".into()),
            pending_label: Some("human-auth-pending".into()),
            comment_on_required: true,
            close_unverified: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CheckMode {
    Off,
    Audit,
    #[default]
    Enforce,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Issue,
    PullRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subject {
    pub login: String,
    #[serde(default)]
    pub github_user_id: Option<i64>,
    #[serde(default)]
    pub is_collaborator: bool,
    #[serde(default)]
    pub is_bot: bool,
    #[serde(default)]
    pub is_app: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TrustState {
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub manually_exempt: bool,
    /// Unix timestamp seconds when trust was granted/verified.
    #[serde(default)]
    pub trusted_at: Option<i64>,
    /// Unix timestamp seconds when trust expires.
    #[serde(default)]
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionInput {
    pub target: TargetKind,
    pub subject: Subject,
    #[serde(default)]
    pub trust: TrustState,
    /// Unix timestamp seconds used for expiry/reverification decisions.
    pub now: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDecision {
    pub required: bool,
    pub allowed: bool,
    pub reason: DecisionReason,
    pub check_mode: CheckMode,
    pub actions: Vec<PolicyAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    PolicyDisabled,
    TargetDisabled,
    CollaboratorExempt,
    BotOrAppExempt,
    ManuallyExempt,
    Trusted,
    TrustExpired,
    ReverificationRequired,
    VerificationRequired,
    AuditOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum PolicyAction {
    AddLabel(String),
    RemoveLabel(String),
    Comment(String),
    CreateCheck { name: String, conclusion: String },
    Close,
}

pub fn decide(policy: &VerificationPolicy, input: &DecisionInput) -> PolicyDecision {
    if policy.check_mode == CheckMode::Off {
        return allow(DecisionReason::PolicyDisabled, policy, vec![]);
    }
    if (input.target == TargetKind::Issue && !policy.verify_issues)
        || (input.target == TargetKind::PullRequest && !policy.verify_pull_requests)
    {
        return allow(DecisionReason::TargetDisabled, policy, vec![]);
    }
    if policy.exempt_collaborators && input.subject.is_collaborator {
        return allow(
            DecisionReason::CollaboratorExempt,
            policy,
            verified_actions(policy),
        );
    }
    if policy.exempt_verified_bots && (input.subject.is_bot || input.subject.is_app) {
        return allow(
            DecisionReason::BotOrAppExempt,
            policy,
            verified_actions(policy),
        );
    }
    if input.trust.manually_exempt {
        return allow(
            DecisionReason::ManuallyExempt,
            policy,
            verified_actions(policy),
        );
    }
    if input.trust.trusted {
        if input.trust.expires_at.is_some_and(|ts| ts <= input.now) {
            return require(DecisionReason::TrustExpired, policy);
        }
        if let (Some(days), Some(trusted_at)) = (policy.reverify_after_days, input.trust.trusted_at)
        {
            if input.now.saturating_sub(trusted_at) >= i64::from(days) * 86_400 {
                return require(DecisionReason::ReverificationRequired, policy);
            }
        }
        return allow(DecisionReason::Trusted, policy, verified_actions(policy));
    }
    if policy.check_mode == CheckMode::Audit {
        let mut d = allow(DecisionReason::AuditOnly, policy, pending_actions(policy));
        d.required = true;
        return d;
    }
    require(DecisionReason::VerificationRequired, policy)
}

fn allow(
    reason: DecisionReason,
    policy: &VerificationPolicy,
    actions: Vec<PolicyAction>,
) -> PolicyDecision {
    PolicyDecision {
        required: false,
        allowed: true,
        reason,
        check_mode: policy.check_mode,
        actions,
    }
}

fn require(reason: DecisionReason, policy: &VerificationPolicy) -> PolicyDecision {
    PolicyDecision {
        required: true,
        allowed: false,
        reason,
        check_mode: policy.check_mode,
        actions: pending_actions(policy),
    }
}

fn verified_actions(policy: &VerificationPolicy) -> Vec<PolicyAction> {
    let mut actions = Vec::new();
    if let Some(label) = &policy.pending_label {
        actions.push(PolicyAction::RemoveLabel(label.clone()));
    }
    if let Some(label) = &policy.verified_label {
        actions.push(PolicyAction::AddLabel(label.clone()));
    }
    actions.push(PolicyAction::CreateCheck {
        name: "Human Auth".into(),
        conclusion: "success".into(),
    });
    actions
}

fn pending_actions(policy: &VerificationPolicy) -> Vec<PolicyAction> {
    let mut actions = Vec::new();
    if let Some(label) = &policy.apply_label {
        actions.push(PolicyAction::AddLabel(label.clone()));
    }
    if let Some(label) = &policy.pending_label {
        actions.push(PolicyAction::AddLabel(label.clone()));
    }
    if policy.comment_on_required {
        actions.push(PolicyAction::Comment(
            "Human verification is required before this contribution can proceed.".into(),
        ));
    }
    actions.push(PolicyAction::CreateCheck {
        name: "Human Auth".into(),
        conclusion: if policy.check_mode == CheckMode::Audit {
            "neutral"
        } else {
            "action_required"
        }
        .into(),
    });
    if policy.close_unverified && policy.check_mode == CheckMode::Enforce {
        actions.push(PolicyAction::Close);
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> DecisionInput {
        DecisionInput {
            target: TargetKind::Issue,
            subject: Subject {
                login: "octo".into(),
                github_user_id: Some(1),
                is_collaborator: false,
                is_bot: false,
                is_app: false,
            },
            trust: TrustState::default(),
            now: 10_000_000,
        }
    }

    #[test]
    fn requires_untrusted_issue() {
        let d = decide(&VerificationPolicy::default(), &input());
        assert!(d.required);
        assert!(!d.allowed);
        assert_eq!(d.reason, DecisionReason::VerificationRequired);
    }

    #[test]
    fn exempts_collaborator() {
        let mut i = input();
        i.subject.is_collaborator = true;
        let d = decide(&VerificationPolicy::default(), &i);
        assert!(d.allowed);
        assert_eq!(d.reason, DecisionReason::CollaboratorExempt);
    }

    #[test]
    fn detects_expired_and_reverify() {
        let mut i = input();
        i.trust = TrustState {
            trusted: true,
            manually_exempt: false,
            trusted_at: Some(0),
            expires_at: Some(9_999_999),
        };
        assert_eq!(
            decide(&VerificationPolicy::default(), &i).reason,
            DecisionReason::TrustExpired
        );
        i.trust.expires_at = None;
        assert_eq!(
            decide(&VerificationPolicy::default(), &i).reason,
            DecisionReason::ReverificationRequired
        );
    }

    #[test]
    fn audit_allows_but_marks_required() {
        let p = VerificationPolicy {
            check_mode: CheckMode::Audit,
            ..Default::default()
        };
        let d = decide(&p, &input());
        assert!(d.allowed && d.required);
        assert_eq!(d.reason, DecisionReason::AuditOnly);
    }
}
