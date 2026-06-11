"use client";

import Link from "next/link";
import { useEffect, useMemo } from "react";
import { VerifyShell } from "./VerifyShell";

export function VerifySuccessClient({ sessionId }: { sessionId: string }) {
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
    window.location.replace(redirectUrl);
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

        {redirectUrl ? (
          <div className="flex flex-wrap items-center gap-3">
            <a
              href={redirectUrl}
              className="inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold text-slate-950 shadow-lg shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200"
            >
              Return to GitHub now
            </a>
            <p className="text-sm text-slate-500">Sending you back to GitHub…</p>
          </div>
        ) : (
          <Link
            href="/"
            className="inline-flex h-11 items-center justify-center rounded-xl border border-white/10 bg-white/5 px-5 text-sm font-semibold text-slate-100 transition hover:border-cyan-300/40 hover:bg-white/10"
          >
            Back home
          </Link>
        )}

        <p className="text-xs text-slate-500">Session {sessionId}</p>
      </div>
    </VerifyShell>
  );
}
