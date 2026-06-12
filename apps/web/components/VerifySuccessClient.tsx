"use client";

import { useEffect, useMemo, useState } from "react";
import { apiFetch, type VerifySession } from "../lib/api";
import { VerifyShell } from "./VerifyShell";

const redirectDelaySeconds = 3;
const countdownCircleRadius = 12;
const countdownCircleCircumference = 2 * Math.PI * countdownCircleRadius;

export function VerifySuccessClient({ sessionId }: { sessionId: string }) {
  const initialParams = useMemo(() => {
    if (typeof window === "undefined") {
      return { redirectUrl: null, login: null, token: null };
    }
    const params = new URLSearchParams(window.location.search);
    return {
      redirectUrl: params.get("redirect"),
      login: params.get("login"),
      token: params.get("token"),
    };
  }, []);

  const [redirectUrl, setRedirectUrl] = useState<string | null>(initialParams.redirectUrl);
  const [login, setLogin] = useState<string | null>(initialParams.login);
  const [secondsRemaining, setSecondsRemaining] = useState(redirectDelaySeconds);
  const [countdownProgress, setCountdownProgress] = useState(100);

  useEffect(() => {
    if (redirectUrl || !initialParams.token) return;

    const query = `?token=${encodeURIComponent(initialParams.token)}`;
    apiFetch<VerifySession>(`/api/verify/${encodeURIComponent(sessionId)}${query}`)
      .then((session) => {
        setRedirectUrl(session.redirect_url ?? session.issue_or_pr_url ?? null);
        setLogin((current) => current ?? session.oauth_login ?? session.github_login ?? null);
      })
      .catch(() => {
        // The success screen can still render; without a verified destination we avoid sending users to github.com.
      });
  }, [initialParams.token, redirectUrl, sessionId]);

  useEffect(() => {
    if (!redirectUrl) return;

    setSecondsRemaining(redirectDelaySeconds);
    setCountdownProgress(100);

    const progressStartId = window.setTimeout(() => setCountdownProgress(0), 50);
    const intervalId = window.setInterval(() => {
      setSecondsRemaining((current) => Math.max(current - 1, 0));
    }, 1000);
    const timeoutId = window.setTimeout(() => {
      window.location.replace(redirectUrl);
    }, redirectDelaySeconds * 1000);

    return () => {
      window.clearTimeout(progressStartId);
      window.clearInterval(intervalId);
      window.clearTimeout(timeoutId);
    };
  }, [redirectUrl]);

  return (
    <VerifyShell
      eyebrow="Verification complete"
      title="You are verified"
      description="Your contributor status has been recorded for this repository."
    >
      <div className="space-y-5">
        <div className="rounded-2xl border border-emerald-400/30 bg-emerald-400/10 px-5 py-4 text-emerald-50">
          <p className="text-lg font-semibold text-white">You should not need to do this again.</p>
          <p className="mt-2 text-sm leading-6 text-emerald-100/90">
            {login ? (
              <>
                <span className="font-semibold text-white">@{login}</span> is now trusted for future issues and pull requests in this repository, unless an admin changes the policy or your trust expires.
              </>
            ) : (
              <>Your GitHub account is now trusted for future issues and pull requests in this repository, unless an admin changes the policy or your trust expires.</>
            )}
          </p>
        </div>

        <p className="text-sm leading-6 text-slate-400">
          The app will update labels, checks, and verification comments on GitHub shortly.
        </p>

        <div className="flex flex-wrap items-center gap-3">
          {redirectUrl ? (
            <>
              <a
                href={redirectUrl}
                className="inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold text-slate-950 shadow-lg shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200"
              >
                Back to GitHub
              </a>
              <div className="flex items-center gap-3 text-sm text-slate-400">
                <span
                  className="relative grid size-9 place-items-center rounded-full text-xs font-bold text-cyan-100"
                  aria-label={`Returning to GitHub in ${secondsRemaining} seconds`}
                >
                  <svg className="absolute inset-0 size-9 -rotate-90" viewBox="0 0 36 36" aria-hidden="true">
                    <circle cx="18" cy="18" r={countdownCircleRadius} fill="none" stroke="rgb(30 41 59)" strokeWidth="4" />
                    <circle
                      cx="18"
                      cy="18"
                      r={countdownCircleRadius}
                      fill="none"
                      stroke="rgb(103 232 249)"
                      strokeLinecap="round"
                      strokeWidth="4"
                      strokeDasharray={countdownCircleCircumference}
                      strokeDashoffset={countdownCircleCircumference * (1 - countdownProgress / 100)}
                      style={{ transition: `stroke-dashoffset ${redirectDelaySeconds}s linear` }}
                    />
                  </svg>
                  <span className="grid size-7 place-items-center rounded-full bg-slate-950">{secondsRemaining}</span>
                </span>
                <span>Returning to the GitHub thread in {secondsRemaining}s</span>
              </div>
            </>
          ) : (
            <p className="text-sm text-slate-400">Preparing your return link to GitHub…</p>
          )}
        </div>

        <p className="text-xs text-slate-500">Session {sessionId}</p>
      </div>
    </VerifyShell>
  );
}
