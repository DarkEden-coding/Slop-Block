"use client";

import Link from "next/link";
import { useMemo } from "react";
import { VerifyShell } from "./VerifyShell";

const ERROR_MESSAGES: Record<string, string> = {
  oauth_denied: "GitHub sign-in was cancelled. Start again when you are ready to verify your account.",
  github_user_mismatch:
    "The GitHub account you signed in with does not match the author of this issue or pull request. Sign in with the correct account and try again.",
};

export function VerifyErrorClient({ sessionId }: { sessionId: string }) {
  const { code, token } = useMemo(() => {
    if (typeof window === "undefined") {
      return { code: null, token: null };
    }
    const params = new URLSearchParams(window.location.search);
    return {
      code: params.get("code"),
      token: params.get("token"),
    };
  }, []);

  const message =
    (code && ERROR_MESSAGES[code]) ||
    "Verification could not be completed. The session may have expired or been cancelled.";

  const retryHref =
    token && sessionId ? `/verify/${sessionId}?token=${encodeURIComponent(token)}` : `/verify/${sessionId}`;

  return (
    <VerifyShell
      eyebrow="Verification failed"
      title="We could not finish verification"
      description="You can retry from the link below or return to the GitHub comment on the issue or pull request."
    >
      <div className="space-y-5">
        <p className="rounded-2xl border border-red-400/30 bg-red-500/10 px-4 py-3 text-sm leading-6 text-red-100">{message}</p>
        <div className="flex flex-wrap gap-3">
          <Link
            href={retryHref}
            className="inline-flex h-11 items-center justify-center rounded-xl bg-white px-5 text-sm font-bold text-slate-950 transition hover:bg-slate-100"
          >
            Try again
          </Link>
          <Link
            href="/"
            className="inline-flex h-11 items-center justify-center rounded-xl border border-white/10 bg-white/5 px-5 text-sm font-semibold text-slate-100 transition hover:border-cyan-300/40 hover:bg-white/10"
          >
            Back home
          </Link>
        </div>
      </div>
    </VerifyShell>
  );
}
