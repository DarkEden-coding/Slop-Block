"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { apiFetch } from "../lib/api";
import { VerifyShell } from "./VerifyShell";

type SourceResponse = {
  session_id?: string | null;
  token?: string | null;
  already_verified: boolean;
  redirect_url: string;
  message: string;
};

export function VerifySourceClient() {
  const [result, setResult] = useState<SourceResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const query = useMemo(() => (typeof window === "undefined" ? "" : window.location.search), []);

  useEffect(() => {
    apiFetch<SourceResponse>(`/api/verify/from-source${query}`)
      .then((response) => {
        setResult(response);
        if (!response.already_verified && response.session_id && response.token) {
          window.location.replace(`/verify/${response.session_id}?token=${encodeURIComponent(response.token)}`);
        }
      })
      .catch((err: Error) => setError(err.message));
  }, [query]);

  if (error) {
    return (
      <VerifyShell title="Verification unavailable" description="This verification link is invalid or could not be prepared.">
        <p className="text-sm text-rose-200">{error}</p>
      </VerifyShell>
    );
  }

  if (!result) {
    return (
      <VerifyShell title="Preparing verification" description="Checking whether you already verified for this repository.">
        <p className="text-slate-300">Loading…</p>
      </VerifyShell>
    );
  }

  if (result.already_verified) {
    return (
      <VerifyShell eyebrow="Already verified" title="You have already verified" description={result.message}>
        <Link className="inline-flex rounded-xl bg-cyan-400 px-4 py-2 font-semibold text-slate-950" href={result.redirect_url}>
          Return to GitHub
        </Link>
      </VerifyShell>
    );
  }

  return (
    <VerifyShell title="Redirecting" description="Opening your human verification flow.">
      <p className="text-slate-300">One moment…</p>
    </VerifyShell>
  );
}
