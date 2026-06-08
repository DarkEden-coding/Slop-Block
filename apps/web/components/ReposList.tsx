"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { apiFetch, type Repo } from "../lib/api";

export function ReposList() {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<Repo[]>("/api/repos")
      .then(setRepos)
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">Loading repositories…</div>;
  if (error) return <div className="rounded-2xl border border-red-400/30 bg-red-950/40 p-6 text-red-100 shadow-xl shadow-black/20">Could not load repositories: {error}</div>;
  if (!repos.length) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">No repositories are available yet.</div>;

  return (
    <div className="overflow-hidden rounded-3xl border border-white/10 bg-white/[0.04] shadow-2xl shadow-black/30 backdrop-blur">
      {repos.map((repo) => (
        <Link key={repo.id} href={`/dashboard/repos/${repo.id}`} className="group flex items-center justify-between gap-4 border-b border-white/10 p-5 transition last:border-b-0 hover:bg-cyan-300/10 sm:gap-5">
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-lg font-bold text-white">{repo.full_name ?? repo.name}</h3>
            <p className="mt-1 text-sm text-slate-400">Configure verification policy</p>
          </div>
          <div className="flex shrink-0 items-center gap-3">
            <span className="inline-flex items-center rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold leading-none text-slate-300">{repo.private ? "Private" : "Public"}</span>
            <span className="inline-flex h-6 w-6 items-center justify-center text-cyan-300 transition group-hover:translate-x-0.5">→</span>
          </div>
        </Link>
      ))}
    </div>
  );
}
