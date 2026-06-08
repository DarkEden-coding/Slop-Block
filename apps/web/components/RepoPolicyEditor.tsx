"use client";

import { useEffect, useState } from "react";
import { apiFetch, defaultPolicy, type RepoPolicy } from "../lib/api";

export function RepoPolicyEditor({ repoId }: { repoId: string }) {
  const [policy, setPolicy] = useState<RepoPolicy>(defaultPolicy);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<RepoPolicy>(`/api/repos/${encodeURIComponent(repoId)}/policy`)
      .then(setPolicy)
      .catch(() => setPolicy(defaultPolicy))
      .finally(() => setLoading(false));
  }, [repoId]);

  async function save() {
    setSaving(true); setError(null); setMessage(null);
    try {
      const saved = await apiFetch<RepoPolicy>(`/api/repos/${encodeURIComponent(repoId)}/policy`, { method: "POST", body: JSON.stringify(policy) });
      setPolicy(saved ?? policy); setMessage("Policy saved.");
    } catch (err) { setError(err instanceof Error ? err.message : "Save failed"); }
    finally { setSaving(false); }
  }

  const toggle = (key: keyof Omit<RepoPolicy, "comment_mode">) => setPolicy((p) => ({ ...p, [key]: !p[key] }));

  if (loading) return <div className="rounded-xl border p-6 text-slate-600">Loading policy…</div>;

  return (
    <div className="rounded-2xl border bg-white p-6 shadow-sm">
      <div className="space-y-4">
        {([
          ["enabled", "Enable verification for this repository"],
          ["require_captcha", "Require CAPTCHA challenge"],
          ["require_oauth", "Require GitHub OAuth account match"],
          ["trusted_contributors_bypass", "Allow trusted contributors to bypass checks"],
        ] as const).map(([key, label]) => (
          <label key={key} className="flex items-center justify-between gap-4 rounded-lg border p-4">
            <span className="font-medium text-slate-800">{label}</span>
            <input type="checkbox" className="h-5 w-5" checked={policy[key]} onChange={() => toggle(key)} />
          </label>
        ))}
        <label className="block rounded-lg border p-4">
          <span className="font-medium text-slate-800">GitHub comment mode</span>
          <select className="mt-2 w-full rounded-md border px-3 py-2" value={policy.comment_mode} onChange={(e) => setPolicy((p) => ({ ...p, comment_mode: e.target.value as RepoPolicy["comment_mode"] }))}>
            <option value="once">Post once per author/session</option><option value="always">Post whenever verification is needed</option><option value="never">Never post comments</option>
          </select>
        </label>
      </div>
      {error && <p className="mt-4 text-sm text-red-700">{error}</p>}{message && <p className="mt-4 text-sm text-green-700">{message}</p>}
      <button onClick={save} disabled={saving} className="mt-6 rounded-lg bg-slate-950 px-5 py-3 font-semibold text-white disabled:opacity-60">{saving ? "Saving…" : "Save policy"}</button>
    </div>
  );
}
