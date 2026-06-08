"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { apiFetch, type VerifySession } from "../lib/api";
import { CaptchaWidget } from "./CaptchaWidget";
import { VerifyShell, VerifyStepIndicator } from "./VerifyShell";

export function VerifyClient({ sessionId }: { sessionId: string }) {
  const router = useRouter();
  const [session, setSession] = useState<VerifySession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submitLockRef = useRef(false);
  const sessionToken = useMemo(() => {
    if (typeof window === "undefined") return null;
    return new URLSearchParams(window.location.search).get("token");
  }, []);

  const oauthHref = useMemo(() => {
    if (typeof window === "undefined") return session?.oauth_url ?? "#";
    const base = session?.oauth_url ?? "/api/github/oauth/start";
    const url = new URL(base, window.location.origin);
    url.searchParams.set("session_id", sessionId);
    if (sessionToken) url.searchParams.set("token", sessionToken);
    return url.toString();
  }, [session, sessionId, sessionToken]);

  const loadSession = useCallback(async () => {
    if (!sessionToken) {
      setError("Verification token is missing. Please reopen the verification link from GitHub.");
      setLoading(false);
      return;
    }
    const query = `?token=${encodeURIComponent(sessionToken)}`;
    const next = await apiFetch<VerifySession>(`/api/verify/${encodeURIComponent(sessionId)}${query}`);
    setSession(next);
    if (next.status === "completed") {
      const redirect = next.redirect_url ?? next.issue_or_pr_url;
      const params = new URLSearchParams({ token: sessionToken });
      if (redirect) params.set("redirect", redirect);
      router.replace(`/verify/${sessionId}/success?${params.toString()}`);
    }
  }, [router, sessionId, sessionToken]);

  useEffect(() => {
    loadSession()
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false));
  }, [loadSession]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const oauthStatus = new URLSearchParams(window.location.search).get("oauth");
    if (oauthStatus === "oauth_verified") {
      loadSession().catch((err: Error) => setError(err.message));
    }
  }, [loadSession]);

  const submitCaptcha = useCallback(
    async (token: string, provider: string | null) => {
      if (!sessionToken || submitLockRef.current) return;
      submitLockRef.current = true;
      setSubmitting(true);
      setError(null);
      try {
        const updated = await apiFetch<VerifySession>(`/api/verify/${encodeURIComponent(sessionId)}/captcha`, {
          method: "POST",
          body: JSON.stringify({
            token,
            session_token: sessionToken,
            provider: provider ?? session?.captcha?.provider,
          }),
        });
        const redirect = updated.redirect_url ?? updated.issue_or_pr_url;
        const params = new URLSearchParams({ token: sessionToken });
        if (redirect) params.set("redirect", redirect);
        if (updated.oauth_login ?? session?.oauth_login) {
          params.set("login", updated.oauth_login ?? session?.oauth_login ?? "");
        }
        router.replace(`/verify/${sessionId}/success?${params.toString()}`);
      } catch (err) {
        submitLockRef.current = false;
        setError(err instanceof Error ? err.message : "CAPTCHA submission failed");
        setSubmitting(false);
      }
    },
    [router, session?.captcha?.provider, session?.oauth_login, sessionId, sessionToken],
  );

  if (loading) {
    return (
      <VerifyShell title="Checking your session" description="Preparing the verification flow for this issue or pull request.">
        <p className="text-slate-300">Loading verification session…</p>
      </VerifyShell>
    );
  }

  if (error && !session) {
    return (
      <VerifyShell title="Verification unavailable" description="This link may be expired, invalid, or already used.">
        <p className="rounded-2xl border border-red-400/30 bg-red-500/10 px-4 py-3 text-red-100">{error}</p>
      </VerifyShell>
    );
  }

  const oauthVerified = session?.oauth_verified === true;
  const currentStep = submitting || session?.status === "completed" ? "done" : oauthVerified ? "captcha" : "oauth";
  const expectedLogin = session?.github_login;
  const signedInLogin = session?.oauth_login;

  return (
    <VerifyShell
      title="Verify you are a human contributor"
      description="We use GitHub sign-in to confirm your username matches the issue or pull request author, then a CAPTCHA to block automated abuse. This check does not request repository write access."
    >
      <VerifyStepIndicator current={currentStep} oauthDone={oauthVerified} />

      {session?.repo ? (
        <p className="mb-6 text-sm text-slate-400">
          Repository: <span className="font-semibold text-slate-200">{session.repo}</span>
          {expectedLogin ? (
            <>
              {" "}
              · Expected author: <span className="font-semibold text-slate-200">@{expectedLogin}</span>
            </>
          ) : null}
        </p>
      ) : null}

      {!oauthVerified ? (
        <div className="space-y-4">
          <p className="text-slate-300">
            First, sign in with GitHub so we can confirm you are the contributor who opened this issue or pull request.
          </p>
          <a
            className="inline-flex h-12 items-center justify-center rounded-xl bg-white px-5 text-sm font-bold text-slate-950 shadow-lg shadow-black/30 transition hover:-translate-y-0.5 hover:bg-slate-100"
            href={oauthHref}
          >
            Continue with GitHub
          </a>
        </div>
      ) : (
        <div className="space-y-6">
          <div className="rounded-2xl border border-emerald-400/20 bg-emerald-400/10 px-4 py-3 text-sm text-emerald-100">
            Signed in as <span className="font-semibold text-white">@{signedInLogin}</span>. Your GitHub username matches this verification request.
          </div>

          {session?.captcha ? (
            <div className="space-y-4">
              <p className="text-slate-300">Complete the CAPTCHA below. We will send you back to GitHub as soon as it is accepted.</p>
              <CaptchaWidget
                config={session.captcha}
                onToken={(token, provider) => {
                  void submitCaptcha(token, provider);
                }}
                onError={setError}
              />
              {submitting ? <p className="text-sm text-cyan-200">Submitting verification…</p> : null}
            </div>
          ) : (
            <p className="rounded-2xl border border-amber-400/30 bg-amber-400/10 px-4 py-3 text-amber-100">
              CAPTCHA is not configured for this deployment.
            </p>
          )}
        </div>
      )}

      {error ? <p className="mt-6 rounded-2xl border border-red-400/30 bg-red-500/10 px-4 py-3 text-sm text-red-100">{error}</p> : null}
    </VerifyShell>
  );
}
