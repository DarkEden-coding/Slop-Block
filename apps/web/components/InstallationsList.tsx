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

  if (loading) return <div className="rounded-xl border p-6 text-slate-600">Loading installations…</div>;
  if (error) return <div className="rounded-xl border border-red-200 bg-red-50 p-6 text-red-800">Could not load installations: {error}</div>;
  if (!items.length) return <div className="rounded-xl border p-6 text-slate-600">No GitHub App installations were found.</div>;

  return (
    <div className="grid gap-4">
      {items.map((item) => (
        <div key={item.id} className="rounded-xl border bg-white p-5 shadow-sm">
          <div className="flex items-center justify-between gap-4">
            <div>
              <h3 className="font-semibold text-slate-950">{item.account_login ?? `Installation ${item.id}`}</h3>
              <p className="text-sm text-slate-600">{item.account_type ?? item.target_type ?? "GitHub account"}</p>
            </div>
            <span className="rounded-full bg-cyan-50 px-3 py-1 text-sm text-cyan-700">#{item.id}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
