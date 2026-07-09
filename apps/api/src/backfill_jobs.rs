use std::time::Duration;
use time::OffsetDateTime;

use crate::{
    github_helpers::github_content_delay_seconds,
    github_subjects::{process_subject, SubjectWork},
    github_tokens::{github_client, installation_token},
    AppState,
};
use db::PgPool;
use github::GitHubApi;
use jobs::JobKind;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct BackfillScanPayload {
    backfill_run_id: i64,
    repository_id: i64,
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    enqueued: Option<i32>,
    #[serde(default)]
    discovered: Option<i32>,
}

#[derive(Deserialize)]
struct BackfillSubjectPayload {
    backfill_run_id: i64,
    backfill_item_id: i64,
}

pub async fn handle_backfill_scan(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: BackfillScanPayload = serde_json::from_value(payload)?;
    let run = db::get_backfill_run(pool, p.backfill_run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("backfill run not found"))?;
    if run.status == "cancelled" {
        return Ok(());
    }
    let repo = db::get_repository(pool, p.repository_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("repository not found"))?;
    let _permit = state
        .installation_gate
        .acquire(repo.installation_id as u64)
        .await;
    let token = installation_token(state, repo.installation_id as u64).await?;
    let gh = github_client(state);
    let page = p.page.unwrap_or(1).max(1);
    let mut enqueued = p.enqueued.unwrap_or(0);
    let mut discovered = p.discovered.unwrap_or(0);
    let phase = p.phase.as_deref().unwrap_or(if run.include_issues {
        "issues"
    } else if run.include_pull_requests {
        "pull_requests"
    } else {
        "done"
    });

    match phase {
        "issues" if run.include_issues => {
            db::mark_backfill_phase(pool, run.id, "scanning", Some("scanning_issues")).await?;
            let issues = gh
                .list_open_issues_page(&token, &repo.owner, &repo.name, page, 100)
                .await?;
            let page_len = issues.len();
            for issue in issues {
                if issue.pull_request.is_some() {
                    continue;
                }
                discovered += 1;
                let url = format!(
                    "https://github.com/{}/issues/{}",
                    repo.full_name, issue.number
                );
                if let Some(item) = db::insert_backfill_item(
                    pool,
                    run.id,
                    repo.repository_id,
                    "issue",
                    &issue.number.to_string(),
                    Some(issue.user.id as i64),
                    Some(&issue.user.login),
                    Some(&url),
                    None,
                )
                .await?
                {
                    enqueue_item(state, pool, run.id, item.id, enqueued).await?;
                    enqueued += 1;
                }
            }
            if page_len >= 100 {
                enqueue_scan_page(
                    pool,
                    run.id,
                    repo.repository_id,
                    "issues",
                    page + 1,
                    enqueued,
                    discovered,
                )
                .await?;
                return Ok(());
            }
            if run.include_pull_requests {
                enqueue_scan_page(
                    pool,
                    run.id,
                    repo.repository_id,
                    "pull_requests",
                    1,
                    enqueued,
                    discovered,
                )
                .await?;
                return Ok(());
            }
        }
        "pull_requests" | "issues" if run.include_pull_requests => {
            db::mark_backfill_phase(pool, run.id, "scanning", Some("scanning_pull_requests"))
                .await?;
            let prs = gh
                .list_open_pull_requests_page(&token, &repo.owner, &repo.name, page, 100)
                .await?;
            let page_len = prs.len();
            for pr in prs {
                discovered += 1;
                let url = format!("https://github.com/{}/pull/{}", repo.full_name, pr.number);
                if let Some(item) = db::insert_backfill_item(
                    pool,
                    run.id,
                    repo.repository_id,
                    "pull_request",
                    &pr.number.to_string(),
                    Some(pr.user.id as i64),
                    Some(&pr.user.login),
                    Some(&url),
                    Some(&pr.head.sha),
                )
                .await?
                {
                    enqueue_item(state, pool, run.id, item.id, enqueued).await?;
                    enqueued += 1;
                }
            }
            if page_len >= 100 {
                enqueue_scan_page(
                    pool,
                    run.id,
                    repo.repository_id,
                    "pull_requests",
                    page + 1,
                    enqueued,
                    discovered,
                )
                .await?;
                return Ok(());
            }
        }
        _ => {}
    }

    // Intermediate scan pages carry cumulative counters in the job payload; write absolute totals once at the end.
    sqlx::query(
        "UPDATE backfill_runs SET total_discovered=$2, total_enqueued=$3, updated_at=now() WHERE id=$1",
    )
    .bind(run.id)
    .bind(discovered)
    .bind(enqueued)
    .execute(pool)
    .await?;
    let status = if enqueued == 0 {
        "completed"
    } else {
        "running"
    };
    db::mark_backfill_phase(
        pool,
        run.id,
        status,
        Some(if enqueued == 0 {
            "finalizing"
        } else {
            "processing"
        }),
    )
    .await?;
    Ok(())
}

async fn enqueue_scan_page(
    pool: &PgPool,
    run_id: i64,
    repository_id: i64,
    phase: &str,
    page: u32,
    enqueued: i32,
    discovered: i32,
) -> anyhow::Result<()> {
    let payload = json!({
        "backfill_run_id": run_id,
        "repository_id": repository_id,
        "phase": phase,
        "page": page,
        "enqueued": enqueued,
        "discovered": discovered,
    });
    let key = format!("backfill:{run_id}:scan:{phase}:{page}");
    jobs::enqueue_deduped(
        pool,
        JobKind::BackfillScan,
        payload,
        None,
        8,
        Some(&key),
        40,
    )
    .await?;
    Ok(())
}

async fn enqueue_item(
    state: &AppState,
    pool: &PgPool,
    run_id: i64,
    item_id: i64,
    sequence: i32,
) -> anyhow::Result<()> {
    let payload = json!({"backfill_run_id": run_id, "backfill_item_id": item_id});
    let key = format!("backfill:{run_id}:item:{item_id}");
    let delay_seconds = github_content_delay_seconds(state) as i64;
    let run_at = OffsetDateTime::now_utc()
        + Duration::from_secs((i64::from(sequence) * delay_seconds) as u64);
    jobs::enqueue_deduped(
        pool,
        JobKind::BackfillSubject,
        payload,
        Some(run_at),
        8,
        Some(&key),
        50,
    )
    .await?;
    Ok(())
}

pub async fn handle_backfill_subject(
    state: &AppState,
    pool: &PgPool,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let p: BackfillSubjectPayload = serde_json::from_value(payload)?;
    let run = db::get_backfill_run(pool, p.backfill_run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("backfill run not found"))?;
    if run.status == "cancelled" {
        db::finish_backfill_item(
            pool,
            run.id,
            p.backfill_item_id,
            "skipped",
            None,
            Some("cancelled"),
            None,
        )
        .await?;
        return Ok(());
    }
    let item = db::mark_backfill_item_running(pool, p.backfill_item_id)
        .await?
        .or(db::get_backfill_item(pool, p.backfill_item_id).await?)
        .ok_or_else(|| anyhow::anyhow!("backfill item not found"))?;
    let number = item.subject_id.parse::<u64>()?;
    let work = SubjectWork {
        repository_id: item.repository_id,
        installation_id: None,
        subject_type: item.subject_type.clone(),
        number,
        html_url: item.html_url.clone().unwrap_or_default(),
        head_sha: item.head_sha.clone(),
        github_user_id: item.github_user_id.unwrap_or_default(),
        login: item.login.clone().unwrap_or_else(|| "unknown".into()),
        user_type: Some("User".into()),
        source: "backfill".into(),
        notify_author: run.notify_authors,
        force_new_comment: run.force_new_comments,
    };
    match process_subject(state, pool, work).await {
        Ok(outcome) => {
            let status = if outcome.skipped || !outcome.required {
                "skipped"
            } else {
                "succeeded"
            };
            db::finish_backfill_item(
                pool,
                run.id,
                item.id,
                status,
                Some(outcome.required),
                Some(&outcome.reason),
                None,
            )
            .await?;
            if outcome.skipped {
                accelerate_next_backfill_item(pool, run.id, 0).await?;
            } else {
                accelerate_next_backfill_item(
                    pool,
                    run.id,
                    github_content_delay_seconds(state) as i64,
                )
                .await?;
            }
            Ok(())
        }
        Err(err) => {
            let error = err.to_string();
            if error.contains("secondary rate limited")
                || error.contains("temporarily blocked from content creation")
                || error.contains("github_rate_limited_until:")
            {
                return Err(err);
            }
            db::finish_backfill_item(pool, run.id, item.id, "failed", None, None, Some(&error))
                .await?;
            Ok(())
        }
    }
}

async fn accelerate_next_backfill_item(
    pool: &PgPool,
    run_id: i64,
    delay_seconds: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE jobs
        SET run_at = now() + ($2::text || ' seconds')::interval,
            updated_at = now()
        WHERE id = (
            SELECT id
            FROM jobs
            WHERE kind = 'backfill_subject'
              AND status = 'queued'
              AND (payload->>'backfill_run_id')::bigint = $1
            ORDER BY run_at, id
            LIMIT 1
        )
        "#,
    )
    .bind(run_id)
    .bind(delay_seconds)
    .execute(pool)
    .await?;
    Ok(())
}
