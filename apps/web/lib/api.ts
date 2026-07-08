import { showToast } from "./toast";

export const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? "";

export type Installation = {
  id: number | string;
  account_login?: string;
  account_type?: string;
  target_type?: string;
  created_at?: string;
};

export type Repo = {
  id: number | string;
  name: string;
  full_name?: string;
  private?: boolean;
  installation_id?: number | string;
};

export type TrustedUser = {
  id: number;
  subject_id: string;
  github_user_id?: number | null;
  login?: string | null;
  reason?: string | null;
  trusted_at?: string;
  expires_at?: string | null;
};

export type CaptchaProviderInfo = {
  id: string;
  label: string;
  site_key?: string | null;
  configured: boolean;
  secret_set: boolean;
  source?: "dashboard" | "environment" | null;
};

export type CaptchaProviderCredentialsUpdate = {
  site_key?: string | null;
  secret?: string | null;
};

export type CaptchaSettingsUpdate = {
  enabled_providers: string[];
  default_provider?: string | null;
  providers?: Record<string, CaptchaProviderCredentialsUpdate>;
};

export type CaptchaSettings = {
  available_providers: CaptchaProviderInfo[];
  enabled_providers: string[];
  default_provider: string | null;
  dev_bypass: boolean;
};

export type CaptchaPublicProvider = {
  id: string;
  label: string;
  site_key: string;
};

export type SessionCaptchaConfig = {
  provider: string;
  site_key: string;
  label: string;
  alternate_providers?: CaptchaPublicProvider[];
};

export type RepoPolicy = {
  enabled: boolean;
  verify_issues: boolean;
  verify_pull_requests: boolean;
  exempt_collaborators: boolean;
  exempt_verified_bots: boolean;
  reverify_after_days: number | null;
  check_mode: "off" | "audit" | "enforce";
  apply_label: string | null;
  verified_label: string | null;
  pending_label: string | null;
  comment_on_required: boolean;
  captcha_provider: string | null;
};

export type RepoPolicyResponse = {
  enabled: boolean;
  policy: unknown;
  trusted_users?: TrustedUser[];
};

export type BackfillRun = {
  id: number;
  status: "queued" | "scanning" | "running" | "completed" | "failed" | "cancelled" | string;
  current_phase?: string | null;
  total_discovered: number;
  total_enqueued: number;
  total_processed: number;
  total_succeeded: number;
  total_failed: number;
  total_skipped: number;
  last_error?: string | null;
  created_at: string;
  started_at?: string | null;
  completed_at?: string | null;
};

export type BackfillRequest = {
  include_issues: boolean;
  include_pull_requests: boolean;
  notify_authors: boolean;
  force_new_comments: boolean;
};

export type QueueJob = {
  id: number;
  kind: string;
  status: string;
  priority: number;
  attempts: number;
  max_attempts: number;
  run_at: string;
  locked_by?: string | null;
  last_error?: string | null;
  subject_type?: string | null;
  subject_number?: number | null;
  source?: string | null;
  backfill_run_id?: number | null;
  available_after_rate_limit: boolean;
  rate_limit_reset_at?: string | null;
};

export type RateLimitPause = {
  bucket: string;
  paused_until: string;
  remaining?: number | null;
  reset_at?: string | null;
  last_error?: string | null;
};

export type PropagationRun = {
  id: number;
  github_user_id?: number | null;
  login?: string | null;
  status: string;
  total_subjects: number;
  processed_subjects: number;
  current_subject_type?: string | null;
  current_subject_id?: string | null;
  last_error?: string | null;
  started_at: string;
  completed_at?: string | null;
};

export type RepoQueueStatus = {
  jobs: QueueJob[];
  backfill: BackfillRun | null;
  rate_limits: RateLimitPause[];
  propagations: PropagationRun[];
  has_active_work: boolean;
};

export type AuthMe = {
  authenticated: boolean;
  user: {
    id: number;
    login: string;
    avatar_url?: string | null;
    html_url?: string | null;
  } | null;
  login_url: string;
};

export type VerifySession = {
  session_id: string;
  status: "pending" | "completed" | "verified" | "failed" | "expired" | string;
  repo?: string;
  github_login?: string;
  issue_or_pr_url?: string;
  redirect_url?: string;
  oauth_url?: string;
  captcha_required?: boolean;
  oauth_required?: boolean;
  oauth_verified?: boolean;
  oauth_login?: string;
  captcha?: SessionCaptchaConfig | null;
  message?: string;
};

function url(path: string) {
  if (!API_BASE_URL) return path;
  return `${API_BASE_URL.replace(/\/$/, "")}${path}`;
}

export function apiUrl(path: string) {
  return url(path);
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url(path), {
      ...init,
      credentials: "include",
      headers: {
        "content-type": "application/json",
        "x-requested-with": "github-human-auth",
        ...(init?.headers ?? {}),
      },
    });
  } catch (err) {
    showToast(`Could not reach the API (${path}).`, "error", "Network error");
    throw err;
  }

  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string | { message?: string }; message?: string };
      message = typeof body.error === "string" ? body.error : body.error?.message ?? body.message ?? message;
    } catch {}
    showToast(`${message} (${path})`, "error", "Request failed");
    throw new Error(message);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const defaultPolicy: RepoPolicy = {
  enabled: true,
  verify_issues: true,
  verify_pull_requests: true,
  exempt_collaborators: false,
  exempt_verified_bots: true,
  reverify_after_days: 90,
  check_mode: "enforce",
  apply_label: "human-auth-required",
  verified_label: "human-auth-verified",
  pending_label: "human-auth-pending",
  comment_on_required: true,
  captcha_provider: null,
};

export const CAPTCHA_PROVIDER_OPTIONS = [
  { id: "cloudflare-turnstile", label: "Cloudflare Turnstile" },
  { id: "hcaptcha", label: "hCaptcha" },
  { id: "google-recaptcha-v2", label: "Google reCAPTCHA v2" },
] as const;
