"use client";

import { useEffect, useState } from "react";
import { apiFetch, type Installation } from "../lib/api";

export function InstallationsList() {
  const [items, setItems] = useState<Installation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<Installation[]>("/api/installations")
      .then(setItems)
      .catch((err: Error) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">Loading installations…</div>;
  if (error) return <div className="rounded-2xl border border-red-400/30 bg-red-950/40 p-6 text-red-100 shadow-xl shadow-black/20">Could not load installations: {error}</div>;
  if (!items.length) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">No GitHub App installations were found.</div>;

  return (
    <div className="overflow-hidden rounded-3xl border border-white/10 bg-white/[0.04] shadow-2xl shadow-black/30 backdrop-blur">
      {items.map((item) => (
        <div key={item.id} className="flex items-center justify-between gap-4 border-b border-white/10 p-5 last:border-b-0">
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-lg font-bold text-white">{item.account_login ?? `Installation ${item.id}`}</h3>
            <p className="mt-1 text-sm text-slate-400">{item.account_type ?? item.target_type ?? "GitHub account"}</p>
          </div>
          <span className="inline-flex shrink-0 items-center rounded-full border border-cyan-300/20 bg-cyan-300/10 px-3 py-1 text-sm font-semibold leading-none text-cyan-200">#{item.id}</span>
        </div>
      ))}
    </div>
  );
}
