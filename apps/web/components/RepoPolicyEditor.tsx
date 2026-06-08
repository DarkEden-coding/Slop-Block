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

  if (loading) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">Loading policy…</div>;

  return (
    <section className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
      <div className="mb-6 rounded-2xl border border-cyan-300/20 bg-cyan-300/10 p-4 text-sm leading-6 text-cyan-100 shadow-lg shadow-cyan-950/20">
        Defaults verify both issues and pull requests and post a GitHub comment with the verification link.
      </div>

      <div className="divide-y divide-white/10 overflow-hidden rounded-2xl border border-white/10">
        {boolFields.map(([key, label, help]) => {
          const checked = Boolean(policy[key]);
          return (
            <label key={key} className="flex cursor-pointer items-center justify-between gap-5 bg-slate-950/40 p-4 transition hover:bg-white/[0.06]">
              <span className="min-w-0">
                <span className="block font-semibold text-white">{label}</span>
                <span className="mt-1 block text-sm text-slate-400">{help}</span>
              </span>
              <span className={`relative h-7 w-12 shrink-0 rounded-full border transition ${checked ? "border-cyan-300/50 bg-cyan-300/80 shadow-lg shadow-cyan-500/20" : "border-white/15 bg-white/10"}`}>
                <input type="checkbox" className="peer sr-only" checked={checked} onChange={() => toggle(key)} />
                <span className={`absolute top-1 h-5 w-5 rounded-full bg-white shadow transition ${checked ? "left-6" : "left-1"}`} />
              </span>
            </label>
          );
        })}
      </div>

      <div className="mt-5 grid gap-4 md:grid-cols-2">
        <label className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
          <span className="font-semibold text-white">Check mode</span>
          <select className="mt-3 w-full rounded-xl border border-white/10 bg-slate-950 px-3 py-3 text-white outline-none ring-cyan-300/40 transition focus:ring-4" value={policy.check_mode} onChange={(e) => setPolicy((p) => ({ ...p, check_mode: e.target.value as RepoPolicy["check_mode"] }))}>
            <option value="enforce">Enforce</option><option value="audit">Audit only</option><option value="off">Off</option>
          </select>
        </label>
        <label className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
          <span className="font-semibold text-white">Reverify after days</span>
          <input type="number" min="1" className="mt-3 w-full rounded-xl border border-white/10 bg-slate-950 px-3 py-3 text-white outline-none ring-cyan-300/40 transition focus:ring-4" value={policy.reverify_after_days ?? ""} onChange={(e) => setPolicy((p) => ({ ...p, reverify_after_days: e.target.value ? Number(e.target.value) : null }))} />
        </label>
      </div>

      <div className="mt-5 grid gap-4 md:grid-cols-3">
        {labelFields.map(([key, label]) => (
          <label key={key} className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
            <span className="font-semibold text-white">{label}</span>
            <input className="mt-3 w-full rounded-xl border border-white/10 bg-slate-950 px-3 py-3 text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-600 focus:ring-4" value={(policy[key] as string | null) ?? ""} onChange={(e) => setText(key, e.target.value)} />
          </label>
        ))}
      </div>

      {error && <p className="mt-4 text-sm font-medium text-red-300">{error}</p>}{message && <p className="mt-4 text-sm font-medium text-emerald-300">{message}</p>}
      <button onClick={save} disabled={saving} className="mt-6 rounded-xl bg-cyan-300 px-6 py-3 font-bold text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200 disabled:translate-y-0 disabled:opacity-60">{saving ? "Saving…" : "Save policy"}</button>
    </section>
  );
}
