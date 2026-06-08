"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { apiFetch, CAPTCHA_PROVIDER_OPTIONS, defaultPolicy, type CaptchaSettings, type RepoPolicy, type RepoPolicyResponse } from "../lib/api";

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
  const [captchaSettings, setCaptchaSettings] = useState<CaptchaSettings | null>(null);
  const [captchaSettingsError, setCaptchaSettingsError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<CaptchaSettings>("/api/settings/captcha")
      .then((loaded) => {
        setCaptchaSettings(loaded);
        setCaptchaSettingsError(null);
      })
      .catch((err: Error) => {
        setCaptchaSettings(null);
        setCaptchaSettingsError(err.message);
      });
  }, []);

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
            <label key={key} className="flex cursor-pointer items-start justify-between gap-5 bg-slate-950/40 p-4 transition hover:bg-white/[0.06] sm:items-center">
              <span className="min-w-0 pr-2">
                <span className="block font-semibold text-white">{label}</span>
                <span className="mt-1 block text-sm leading-relaxed text-slate-400">{help}</span>
              </span>
              <span className={`relative mt-0.5 h-7 w-12 shrink-0 rounded-full border transition sm:mt-0 ${checked ? "border-cyan-300/50 bg-cyan-300/80 shadow-lg shadow-cyan-500/20" : "border-white/15 bg-white/10"}`}>
                <input type="checkbox" className="peer sr-only" checked={checked} onChange={() => toggle(key)} />
                <span className={`absolute top-1 h-5 w-5 rounded-full bg-white shadow transition ${checked ? "left-6" : "left-1"}`} />
              </span>
            </label>
          );
        })}
      </div>

      <div className="mt-5 grid gap-4 md:grid-cols-2">
        <label className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
          <span className="font-semibold text-white">CAPTCHA provider</span>
          <p className="mt-1 text-sm text-slate-400">
            Optional per-repository override. Set up providers first in{" "}
            <Link href="/dashboard/settings" className="font-semibold text-cyan-300 hover:text-cyan-100">
              CAPTCHA settings
            </Link>
            .
          </p>
          <select
            className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition focus:ring-4"
            value={policy.captcha_provider ?? ""}
            onChange={(e) => setPolicy((p) => ({ ...p, captcha_provider: e.target.value ? e.target.value : null }))}
          >
            <option value="">
              Use installation default
              {captchaSettings?.default_provider
                ? ` (${captchaSettings.available_providers.find((provider) => provider.id === captchaSettings.default_provider)?.label ?? captchaSettings.default_provider})`
                : ""}
            </option>
            {(captchaSettings?.enabled_providers ?? []).map((providerId) => {
              const label = captchaSettings?.available_providers.find((provider) => provider.id === providerId)?.label
                ?? CAPTCHA_PROVIDER_OPTIONS.find((option) => option.id === providerId)?.label
                ?? providerId;
              return (
                <option key={providerId} value={providerId}>
                  {label}
                </option>
              );
            })}
          </select>
          {captchaSettingsError && (
            <p className="mt-2 text-sm text-amber-300">Could not load installation CAPTCHA settings: {captchaSettingsError}</p>
          )}
          {!captchaSettingsError && captchaSettings && captchaSettings.enabled_providers.length === 0 && (
            <p className="mt-2 text-sm text-amber-300">
              No providers are enabled yet. Open{" "}
              <Link href="/dashboard/settings" className="font-semibold text-cyan-200 hover:text-white">
                CAPTCHA settings
              </Link>{" "}
              to add your Turnstile site key and enable a provider.
            </p>
          )}
        </label>
        <label className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
          <span className="font-semibold text-white">Check mode</span>
          <select className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition focus:ring-4" value={policy.check_mode} onChange={(e) => setPolicy((p) => ({ ...p, check_mode: e.target.value as RepoPolicy["check_mode"] }))}>
            <option value="enforce">Enforce</option><option value="audit">Audit only</option><option value="off">Off</option>
          </select>
        </label>
        <label className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
          <span className="font-semibold text-white">Reverify after days</span>
          <input type="number" min="1" className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition focus:ring-4" value={policy.reverify_after_days ?? ""} onChange={(e) => setPolicy((p) => ({ ...p, reverify_after_days: e.target.value ? Number(e.target.value) : null }))} />
        </label>
      </div>

      <div className="mt-5 grid gap-4 md:grid-cols-3">
        {labelFields.map(([key, label]) => (
          <label key={key} className="block rounded-2xl border border-white/10 bg-slate-950/40 p-4">
            <span className="font-semibold text-white">{label}</span>
            <input className="mt-3 h-11 w-full rounded-xl border border-white/10 bg-slate-950 px-3 text-sm text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-600 focus:ring-4" value={(policy[key] as string | null) ?? ""} onChange={(e) => setText(key, e.target.value)} />
          </label>
        ))}
      </div>

      {error && <p className="mt-4 text-sm font-medium text-red-300">{error}</p>}{message && <p className="mt-4 text-sm font-medium text-emerald-300">{message}</p>}
      <button onClick={save} disabled={saving} className="mt-6 inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-6 text-sm font-bold leading-none text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200 disabled:translate-y-0 disabled:opacity-60">{saving ? "Saving…" : "Save policy"}</button>
    </section>
  );
}
