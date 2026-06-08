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

export type CaptchaPublicConfig = {
  providers: CaptchaPublicProvider[];
  default_provider: string | null;
  dev_bypass: boolean;
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
  close_unverified: boolean;
  captcha_provider: string | null;
};

export type RepoPolicyResponse = {
  enabled: boolean;
  policy: unknown;
  trusted_users?: TrustedUser[];
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

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url(path), {
    ...init,
    credentials: "include",
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string | { message?: string }; message?: string };
      message = typeof body.error === "string" ? body.error : body.error?.message ?? body.message ?? message;
    } catch {}
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
  close_unverified: false,
  captcha_provider: null,
};

export const CAPTCHA_PROVIDER_OPTIONS = [
  { id: "cloudflare-turnstile", label: "Cloudflare Turnstile" },
  { id: "hcaptcha", label: "hCaptcha" },
  { id: "google-recaptcha-v2", label: "Google reCAPTCHA v2" },
] as const;
