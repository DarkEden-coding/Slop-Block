"use client";

import { useEffect, useMemo, useState } from "react";
import { apiFetch, type BackfillRequest, type BackfillRun } from "../lib/api";
import { showSuccessToast } from "../lib/toast";

type Props = { repoId: string };

const active = new Set(["queued", "scanning", "running"]);

export function BackfillPanel({ repoId }: Props) {
  const [run, setRun] = useState<BackfillRun | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<BackfillRequest>({
    include_issues: true,
    include_pull_requests: true,
    notify_authors: true,
    force_new_comments: false,
  });

  async function load() {
    const current = await apiFetch<BackfillRun | null>(`/api/repos/${repoId}/backfills/current`);
    setRun(current);
  }

  useEffect(() => {
    load().finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoId]);

  useEffect(() => {
    if (!run || !active.has(run.status)) return;
    const id = window.setInterval(() => void load(), 2000);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoId, run?.status]);

  const denominator = Math.max(run?.total_enqueued ?? 0, run?.total_discovered ?? 0, 1);
  const percent = Math.min(100, Math.round(((run?.total_processed ?? 0) / denominator) * 100));
  const canStart = !run || !active.has(run.status);

  async function start() {
    setSaving(true);
    try {
      const created = await apiFetch<BackfillRun>(`/api/repos/${repoId}/backfills`, {
        method: "POST",
        body: JSON.stringify(form),
      });
      setRun(created);
      showSuccessToast("Backfill was queued.");
    } finally {
      setSaving(false);
    }
  }

  async function cancel() {
    if (!run) return;
    setSaving(true);
    try {
      const cancelled = await apiFetch<BackfillRun>(`/api/repos/${repoId}/backfills/${run.id}/cancel`, { method: "POST" });
      setRun(cancelled);
      showSuccessToast("Backfill was cancelled.");
    } finally {
      setSaving(false);
    }
  }

  const statusText = useMemo(() => {
    if (loading) return "Loading…";
    if (!run) return "No backfill has run yet.";
    return `${run.status}${run.current_phase ? ` · ${run.current_phase.replaceAll("_", " ")}` : ""}`;
  }, [loading, run]);

  return (
    <section className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Backfill existing issues and PRs</h2>
          <p className="mt-2 max-w-2xl text-sm text-slate-400">
            Queue verification labels, comments, and PR checks for open items that existed before policy was enabled. Webhooks remain higher priority than backfill work.
          </p>
        </div>
        <div className="flex gap-2">
          {!canStart && <button disabled={saving} onClick={cancel} className="rounded-xl border border-red-400/30 px-4 py-2 text-sm font-semibold text-red-100 hover:bg-red-950/40 disabled:opacity-60">Cancel</button>}
          <button disabled={!canStart || saving || (!form.include_issues && !form.include_pull_requests)} onClick={start} className="rounded-xl bg-cyan-300 px-4 py-2 text-sm font-bold text-slate-950 hover:bg-cyan-200 disabled:opacity-60">Start backfill</button>
        </div>
      </div>

      <div className="mt-5 grid gap-3 sm:grid-cols-2">
        <label className="flex items-center gap-3 text-sm text-slate-300"><input type="checkbox" checked={form.include_issues} onChange={(e) => setForm((f) => ({ ...f, include_issues: e.target.checked }))} /> Include open issues</label>
        <label className="flex items-center gap-3 text-sm text-slate-300"><input type="checkbox" checked={form.include_pull_requests} onChange={(e) => setForm((f) => ({ ...f, include_pull_requests: e.target.checked }))} /> Include open pull requests</label>
        <label className="flex items-center gap-3 text-sm text-slate-300"><input type="checkbox" checked={form.notify_authors} onChange={(e) => setForm((f) => ({ ...f, notify_authors: e.target.checked }))} /> Notify authors with @mention</label>
        <label className="flex items-center gap-3 text-sm text-slate-300"><input type="checkbox" checked={form.force_new_comments} onChange={(e) => setForm((f) => ({ ...f, force_new_comments: e.target.checked }))} /> Force new comments / re-notify</label>
      </div>
      {form.force_new_comments && <p className="mt-3 rounded-xl border border-amber-300/30 bg-amber-950/30 p-3 text-sm text-amber-100">Force new comments can notify many users and create a large number of GitHub notifications.</p>}

      <div className="mt-6 rounded-2xl border border-white/10 bg-slate-950/50 p-4">
        <div className="flex items-center justify-between gap-3 text-sm"><span className="font-semibold text-slate-200">{statusText}</span><span className="text-slate-400">{percent}%</span></div>
        <div className="mt-3 h-2 overflow-hidden rounded-full bg-white/10"><div className="h-full rounded-full bg-cyan-300 transition-all" style={{ width: `${percent}%` }} /></div>
        {run && <div className="mt-4 grid gap-2 text-sm text-slate-300 sm:grid-cols-3">
          <span>Discovered: {run.total_discovered}</span>
          <span>Processed: {run.total_processed} / {run.total_enqueued}</span>
          <span>Succeeded: {run.total_succeeded}</span>
          <span>Skipped: {run.total_skipped}</span>
          <span>Failed: {run.total_failed}</span>
          {run.last_error && <span className="text-red-200 sm:col-span-3">Last error: {run.last_error}</span>}
        </div>}
      </div>
    </section>
  );
}
