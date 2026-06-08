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

  if (loading) return <div className="rounded-xl border p-6 text-slate-600">Loading repositories…</div>;
  if (error) return <div className="rounded-xl border border-red-200 bg-red-50 p-6 text-red-800">Could not load repositories: {error}</div>;
  if (!repos.length) return <div className="rounded-xl border p-6 text-slate-600">No repositories are available yet.</div>;

  return (
    <div className="grid gap-4 md:grid-cols-2">
      {repos.map((repo) => (
        <Link key={repo.id} href={`/dashboard/repos/${repo.id}`} className="rounded-xl border bg-white p-5 shadow-sm transition hover:border-cyan-300 hover:shadow-md">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h3 className="font-semibold text-slate-950">{repo.full_name ?? repo.name}</h3>
              <p className="mt-1 text-sm text-slate-600">Configure verification policy</p>
            </div>
            <span className="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-700">{repo.private ? "Private" : "Public"}</span>
          </div>
        </Link>
      ))}
    </div>
  );
}
