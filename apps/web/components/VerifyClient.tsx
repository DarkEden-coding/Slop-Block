"use client";

import Script from "next/script";
import { useEffect, useMemo, useState } from "react";
import { apiFetch, type VerifySession } from "../lib/api";

declare global { interface Window { turnstile?: { render: (el: string, opts: { sitekey: string; callback: (token: string) => void }) => string } } }

export function VerifyClient({ sessionId }: { sessionId: string }) {
  const [session, setSession] = useState<VerifySession | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [captchaToken, setCaptchaToken] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const siteKey = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY;
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

  useEffect(() => {
    const query = sessionToken ? `?token=${encodeURIComponent(sessionToken)}` : "";
    apiFetch<VerifySession>(`/api/verify/${encodeURIComponent(sessionId)}${query}`)
      .then(setSession).catch((err: Error) => setError(err.message)).finally(() => setLoading(false));
  }, [sessionId, sessionToken]);

  function renderTurnstile() {
    if (siteKey && window.turnstile) window.turnstile.render("#turnstile-widget", { sitekey: siteKey, callback: setCaptchaToken });
  }

  async function submitCaptcha() {
    if (!captchaToken || !sessionToken) {
      setError("Verification token is missing. Please reopen the verification link from GitHub.");
      return;
    }
    setSubmitting(true); setError(null);
    try {
      const updated = await apiFetch<VerifySession>(`/api/verify/${encodeURIComponent(sessionId)}/captcha`, { method: "POST", body: JSON.stringify({ token: captchaToken, session_token: sessionToken }) });
      setSession(updated);
    } catch (err) { setError(err instanceof Error ? err.message : "CAPTCHA submission failed"); }
    finally { setSubmitting(false); }
  }

  if (loading) return <div className="rounded-xl border bg-white p-6">Checking verification session…</div>;
  if (error && !session) return <div className="rounded-xl border border-red-200 bg-red-50 p-6 text-red-800">{error}</div>;

  return (
    <div className="rounded-2xl border bg-white p-6 shadow-sm">
      <h1 className="text-2xl font-bold text-slate-950">Verify you are a human contributor</h1>
      <p className="mt-3 text-slate-600">We only use this check to reduce automated abuse in GitHub issues and pull requests. We do not ask for repository write access, and CAPTCHA/OAuth results are used solely for this verification session.</p>
      {session?.repo && <p className="mt-4 text-sm text-slate-500">Repository: {session.repo}</p>}
      <div className="mt-6 space-y-4">
        {session?.oauth_required !== false && <a className="inline-block rounded-lg bg-slate-950 px-5 py-3 font-semibold text-white" href={oauthHref}>Continue with GitHub OAuth</a>}
        {siteKey && <><Script src="https://challenges.cloudflare.com/turnstile/v0/api.js" onLoad={renderTurnstile} /><div id="turnstile-widget" className="min-h-16" /><button onClick={submitCaptcha} disabled={!captchaToken || submitting} className="rounded-lg bg-cyan-600 px-5 py-3 font-semibold text-white disabled:opacity-60">{submitting ? "Submitting…" : "Submit CAPTCHA"}</button></>}
        {!siteKey && <p className="rounded-lg bg-amber-50 p-4 text-amber-800">CAPTCHA is not configured for this deployment.</p>}
      </div>
      {error && <p className="mt-4 text-sm text-red-700">{error}</p>}
      {session?.status && <p className="mt-6 text-sm text-slate-600">Current status: <strong>{session.status}</strong></p>}
    </div>
  );
}
