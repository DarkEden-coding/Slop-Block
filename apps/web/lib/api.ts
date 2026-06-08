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

export type RepoPolicy = {
  enabled: boolean;
  require_captcha: boolean;
  require_oauth: boolean;
  trusted_contributors_bypass: boolean;
  comment_mode: "always" | "once" | "never";
};

export type RepoPolicyResponse = {
  enabled: boolean;
  policy: unknown;
  trusted_users?: TrustedUser[];
};

export type VerifySession = {
  session_id: string;
  status: "pending" | "verified" | "failed" | "expired" | string;
  repo?: string;
  github_login?: string;
  issue_or_pr_url?: string;
  oauth_url?: string;
  captcha_required?: boolean;
  oauth_required?: boolean;
  message?: string;
};

function url(path: string) {
  if (!API_BASE_URL) return path;
  return `${API_BASE_URL.replace(/\/$/, "")}${path}`;
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url(path), {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string; message?: string };
      message = body.error ?? body.message ?? message;
    } catch {}
    throw new Error(message);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const defaultPolicy: RepoPolicy = {
  enabled: true,
  require_captcha: true,
  require_oauth: true,
  trusted_contributors_bypass: true,
  comment_mode: "once",
};
