"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { apiFetch, type Repo } from "../lib/api";

export function InstallCompleteClient({ installationId, setupAction }: { installationId: string; setupAction: string }) {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [status, setStatus] = useState("Preparing setup…");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!installationId) {
      setError("GitHub did not provide an installation_id.");
      return;
    }
    let cancelled = false;
    async function run() {
      try {
        setStatus("Claiming installation…");
        await apiFetch(`/api/installations/${encodeURIComponent(installationId)}/claim`, { method: "POST" });
        setStatus("Syncing repositories…");
        const synced = await apiFetch<Repo[]>(`/api/installations/${encodeURIComponent(installationId)}/sync`, { method: "POST" });
        if (!cancelled) {
          setRepos(synced);
          setStatus("Setup is ready.");
        }
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    }
    run();
    return () => { cancelled = true; };
  }, [installationId]);

  if (error) return <div className="mt-8 rounded-2xl border border-red-400/30 bg-red-950/40 p-5 text-red-100">{error}</div>;

  return (
    <section className="mt-8 rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30">
      <p className="text-sm text-slate-400">Installation #{installationId} {setupAction ? `(${setupAction})` : null}</p>
      <h2 className="mt-2 text-2xl font-bold">{status}</h2>
      {repos.length > 0 && (
        <div className="mt-6 space-y-3">
          {repos.map((repo) => (
            <Link key={repo.id} href={`/dashboard/repos/${repo.id}`} className="block rounded-2xl border border-white/10 bg-slate-950/70 p-4 transition hover:bg-cyan-300/10">
              <span className="font-semibold text-white">{repo.full_name ?? repo.name}</span>
              <span className="ml-3 text-sm text-slate-400">Configure policy →</span>
            </Link>
          ))}
        </div>
      )}
      <Link href="/dashboard" className="mt-6 inline-flex rounded-xl border border-cyan-300/20 bg-cyan-300/10 px-5 py-3 text-sm font-bold text-cyan-100">Open dashboard</Link>
    </section>
  );
}
