"use client";

import { useEffect, useState } from "react";
import { apiFetch, defaultPolicy, type RepoPolicy, type RepoPolicyResponse } from "../lib/api";

const boolFields = [
  ["enabled", "Enable verification for this repository", "Master switch for this repository."],
  ["verify_issues", "Require verification on issues", "New/reopened issues will receive the verification comment by default."],
  ["verify_pull_requests", "Require verification on pull requests", "New/reopened/synchronized PRs will receive labels, comments, and checks."],
  ["comment_on_required", "Post verification comment", "Adds or updates the GitHub issue/PR comment with the verification link."],
  ["exempt_collaborators", "Exempt collaborators", "Users with write/maintain/admin access bypass verification."],
  ["exempt_verified_bots", "Exempt verified bots/apps", "Bot and GitHub App actors bypass verification."],
  ["close_unverified", "Close unverified contributions", "Only applies in enforce mode."],
] as const satisfies readonly (readonly [keyof RepoPolicy, string, string])[];

const labelFields = [
  ["apply_label", "Required label"],
  ["pending_label", "Pending label"],
  ["verified_label", "Verified label"],
] as const satisfies readonly (readonly [keyof RepoPolicy, string])[];

export function RepoPolicyEditor({ repoId }: { repoId: string }) {
  const [policy, setPolicy] = useState<RepoPolicy>(defaultPolicy);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<RepoPolicyResponse>(`/api/repos/${encodeURIComponent(repoId)}/policy`)
      .then((res) => setPolicy({ ...defaultPolicy, ...(res.policy as Partial<RepoPolicy>), enabled: res.enabled }))
      .catch(() => setPolicy(defaultPolicy))
      .finally(() => setLoading(false));
  }, [repoId]);

  async function save() {
    setSaving(true); setError(null); setMessage(null);
    try {
      const { enabled, ...policyBody } = policy;
      const saved = await apiFetch<RepoPolicyResponse>(`/api/repos/${encodeURIComponent(repoId)}/policy`, { method: "POST", body: JSON.stringify({ enabled, policy: policyBody }) });
      setPolicy({ ...defaultPolicy, ...(saved.policy as Partial<RepoPolicy>), enabled: saved.enabled }); setMessage("Policy saved.");
    } catch (err) { setError(err instanceof Error ? err.message : "Save failed"); }
    finally { setSaving(false); }
  }

  const toggle = (key: keyof RepoPolicy) => setPolicy((p) => ({ ...p, [key]: !p[key] }));
  const setText = (key: keyof RepoPolicy, value: string) => setPolicy((p) => ({ ...p, [key]: value.trim() ? value : null }));

  if (loading) return <div className="rounded-xl border p-6 text-slate-600">Loading policy…</div>;

  return (
    <div className="rounded-2xl border bg-white p-6 shadow-sm">
      <div className="mb-5 rounded-lg bg-cyan-50 p-4 text-sm text-cyan-900">
        Defaults verify both issues and pull requests and post a GitHub comment with the verification link.
      </div>
      <div className="space-y-4">
        {boolFields.map(([key, label, help]) => (
          <label key={key} className="flex items-center justify-between gap-4 rounded-lg border p-4">
            <span><span className="block font-medium text-slate-800">{label}</span><span className="text-sm text-slate-500">{help}</span></span>
            <input type="checkbox" className="h-5 w-5" checked={Boolean(policy[key])} onChange={() => toggle(key)} />
          </label>
        ))}
        <label className="block rounded-lg border p-4">
          <span className="font-medium text-slate-800">Check mode</span>
          <select className="mt-2 w-full rounded-md border px-3 py-2" value={policy.check_mode} onChange={(e) => setPolicy((p) => ({ ...p, check_mode: e.target.value as RepoPolicy["check_mode"] }))}>
            <option value="enforce">Enforce</option><option value="audit">Audit only</option><option value="off">Off</option>
          </select>
        </label>
        <label className="block rounded-lg border p-4">
          <span className="font-medium text-slate-800">Reverify after days</span>
          <input type="number" min="1" className="mt-2 w-full rounded-md border px-3 py-2" value={policy.reverify_after_days ?? ""} onChange={(e) => setPolicy((p) => ({ ...p, reverify_after_days: e.target.value ? Number(e.target.value) : null }))} />
        </label>
        {labelFields.map(([key, label]) => (
          <label key={key} className="block rounded-lg border p-4">
            <span className="font-medium text-slate-800">{label}</span>
            <input className="mt-2 w-full rounded-md border px-3 py-2" value={(policy[key] as string | null) ?? ""} onChange={(e) => setText(key, e.target.value)} />
          </label>
        ))}
      </div>
      {error && <p className="mt-4 text-sm text-red-700">{error}</p>}{message && <p className="mt-4 text-sm text-green-700">{message}</p>}
      <button onClick={save} disabled={saving} className="mt-6 rounded-lg bg-slate-950 px-5 py-3 font-semibold text-white disabled:opacity-60">{saving ? "Saving…" : "Save policy"}</button>
    </div>
  );
}
