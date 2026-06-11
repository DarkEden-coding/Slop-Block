"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { VerifyShell } from "./VerifyShell";

const redirectDelaySeconds = 3;

export function VerifySuccessClient({ sessionId }: { sessionId: string }) {
  const [secondsRemaining, setSecondsRemaining] = useState(redirectDelaySeconds);
  const { redirectUrl, login } = useMemo(() => {
    if (typeof window === "undefined") {
      return { redirectUrl: null, login: null };
    }
    const params = new URLSearchParams(window.location.search);
    return {
      redirectUrl: params.get("redirect"),
      login: params.get("login"),
    };
  }, []);

  useEffect(() => {
    if (!redirectUrl) return;

    setSecondsRemaining(redirectDelaySeconds);
    const intervalId = window.setInterval(() => {
      setSecondsRemaining((current) => Math.max(current - 1, 0));
    }, 1000);
    const timeoutId = window.setTimeout(() => {
      window.location.replace(redirectUrl);
    }, redirectDelaySeconds * 1000);

    return () => {
      window.clearInterval(intervalId);
      window.clearTimeout(timeoutId);
    };
  }, [redirectUrl]);

  const githubHref = redirectUrl ?? "https://github.com";
  const countdownProgress = ((redirectDelaySeconds - secondsRemaining) / redirectDelaySeconds) * 100;

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
          <Link
            href={githubHref}
            className="inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold text-slate-950 shadow-lg shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200"
          >
            Back to GitHub
          </Link>
          {redirectUrl ? (
            <div className="flex items-center gap-3 text-sm text-slate-400">
              <span
                className="grid size-9 place-items-center rounded-full text-xs font-bold text-cyan-100"
                style={{
                  background: `conic-gradient(rgb(103 232 249) ${countdownProgress}%, rgb(30 41 59) ${countdownProgress}%)`,
                }}
                aria-label={`Returning to GitHub in ${secondsRemaining} seconds`}
              >
                <span className="grid size-7 place-items-center rounded-full bg-slate-950">{secondsRemaining}</span>
              </span>
              <span>Returning to GitHub in {secondsRemaining}s</span>
            </div>
          ) : null}
        </div>

        <p className="text-xs text-slate-500">Session {sessionId}</p>
      </div>
    </VerifyShell>
  );
}
