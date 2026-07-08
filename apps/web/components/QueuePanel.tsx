"use client";

import { useEffect, useMemo, useState } from "react";
import { apiFetch, type BackfillRun, type RepoQueueStatus } from "../lib/api";
import { showSuccessToast } from "../lib/toast";

type Props = { repoId: string };

const activeBackfill = new Set(["queued", "scanning", "running"]);

function kindLabel(kind: string) {
  switch (kind) {
    case "github_subject_event":
      return "Subject enforcement";
    case "backfill_scan":
      return "Backfill scan";
    case "backfill_subject":
      return "Backfill item";
    default:
      return kind.replaceAll("_", " ");
  }
}

function statusBadgeClass(status: string) {
  switch (status) {
    case "running":
      return "border-cyan-300/30 bg-cyan-950/40 text-cyan-100";
    case "queued":
      return "border-amber-300/30 bg-amber-950/40 text-amber-100";
    case "failed":
      return "border-red-300/30 bg-red-950/40 text-red-100";
    default:
      return "border-white/10 bg-white/5 text-slate-200";
  }
}

function formatSubject(job: RepoQueueStatus["jobs"][number]) {
  if (job.subject_type && job.subject_number) {
    return `${job.subject_type.replaceAll("_", " ")} #${job.subject_number}`;
  }
  if (job.backfill_run_id) {
    return `backfill run #${job.backfill_run_id}`;
  }
  return "—";
}

export function QueuePanel({ repoId }: Props) {
  const [queue, setQueue] = useState<RepoQueueStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [cancelling, setCancelling] = useState(false);

  async function load() {
    const status = await apiFetch<RepoQueueStatus>(`/api/repos/${repoId}/queue`);
    setQueue(status);
  }

  useEffect(() => {
    load().finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoId]);

  useEffect(() => {
    if (!queue?.has_active_work) return;
    const id = window.setInterval(() => void load(), 2000);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoId, queue?.has_active_work]);

  const backfill = queue?.backfill ?? null;
  const backfillActive = backfill ? activeBackfill.has(backfill.status) : false;
  const backfillPercent = useMemo(() => {
    if (!backfill) return 0;
    const denominator = Math.max(backfill.total_enqueued, backfill.total_discovered, 1);
    return Math.min(100, Math.round((backfill.total_processed / denominator) * 100));
  }, [backfill]);

  async function cancelBackfill() {
    if (!backfill) return;
    setCancelling(true);
    try {
      await apiFetch<BackfillRun>(`/api/repos/${repoId}/backfills/${backfill.id}/cancel`, { method: "POST" });
      showSuccessToast("Backfill was cancelled.");
      await load();
    } finally {
      setCancelling(false);
    }
  }

  const summary = useMemo(() => {
    if (loading) return "Loading queue status…";
    if (!queue?.has_active_work) return "No queued work right now.";
    const parts = [
      queue.jobs.length ? `${queue.jobs.length} job${queue.jobs.length === 1 ? "" : "s"}` : null,
      backfillActive ? "backfill running" : null,
      queue.propagations.length
        ? `${queue.propagations.length} propagation${queue.propagations.length === 1 ? "" : "s"}`
        : null,
      queue.rate_limits.length ? "GitHub rate limit pause" : null,
    ].filter(Boolean);
    return parts.join(" · ");
  }, [loading, queue, backfillActive]);

  return (
    <section className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Queued work</h2>
          <p className="mt-2 max-w-2xl text-sm text-slate-400">
            Live view of background jobs, active backfills, verification propagation, and GitHub rate-limit pauses for this repository.
          </p>
        </div>
        <div className="text-sm font-semibold text-slate-300">{summary}</div>
      </div>

      {queue?.rate_limits.length ? (
        <div className="mt-6 rounded-2xl border border-amber-300/30 bg-amber-950/20 p-4">
          <h3 className="text-sm font-bold uppercase tracking-[0.2em] text-amber-200">GitHub rate limit pause</h3>
          <div className="mt-3 space-y-2">
            {queue.rate_limits.map((pause) => (
              <div key={pause.bucket} className="text-sm text-amber-50">
                <span className="font-semibold">{pause.bucket}</span>
                <span className="text-amber-100/80"> · paused until {new Date(pause.paused_until).toLocaleString()}</span>
                {pause.last_error && <p className="mt-1 text-amber-100/70">{pause.last_error}</p>}
              </div>
            ))}
          </div>
        </div>
      ) : null}

      {backfillActive && backfill ? (
        <div className="mt-6 rounded-2xl border border-white/10 bg-slate-950/50 p-4">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h3 className="text-sm font-bold uppercase tracking-[0.2em] text-slate-300">Backfill</h3>
              <p className="mt-1 text-sm text-slate-300">
                {backfill.status}
                {backfill.current_phase ? ` · ${backfill.current_phase.replaceAll("_", " ")}` : ""}
              </p>
            </div>
            <button
              disabled={cancelling}
              onClick={cancelBackfill}
              className="rounded-xl border border-red-400/30 px-4 py-2 text-sm font-semibold text-red-100 hover:bg-red-950/40 disabled:opacity-60"
            >
              Cancel backfill
            </button>
          </div>
          <div className="mt-4 flex items-center justify-between gap-3 text-sm">
            <span className="text-slate-300">
              Processed {backfill.total_processed} / {Math.max(backfill.total_enqueued, backfill.total_discovered)}
            </span>
            <span className="text-slate-400">{backfillPercent}%</span>
          </div>
          <div className="mt-3 h-2 overflow-hidden rounded-full bg-white/10">
            <div className="h-full rounded-full bg-cyan-300 transition-all" style={{ width: `${backfillPercent}%` }} />
          </div>
          {backfill.last_error && <p className="mt-3 text-sm text-red-200">Last error: {backfill.last_error}</p>}
        </div>
      ) : null}

      {queue?.propagations.length ? (
        <div className="mt-6 rounded-2xl border border-white/10 bg-slate-950/50 p-4">
          <h3 className="text-sm font-bold uppercase tracking-[0.2em] text-slate-300">Verification propagation</h3>
          <div className="mt-3 space-y-3">
            {queue.propagations.map((run) => {
              const total = Math.max(run.total_subjects, 1);
              const percent = Math.min(100, Math.round((run.processed_subjects / total) * 100));
              return (
                <div key={run.id} className="rounded-xl border border-white/10 bg-white/[0.03] p-3">
                  <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
                    <span className="font-semibold text-slate-100">{run.login ?? `user ${run.github_user_id ?? "unknown"}`}</span>
                    <span className={`rounded-full border px-2 py-0.5 text-xs font-semibold ${statusBadgeClass(run.status)}`}>
                      {run.status}
                    </span>
                  </div>
                  <p className="mt-2 text-sm text-slate-400">
                    {run.processed_subjects} / {run.total_subjects || "?"} subjects
                    {run.current_subject_type && run.current_subject_id
                      ? ` · current ${run.current_subject_type.replaceAll("_", " ")} #${run.current_subject_id}`
                      : ""}
                  </p>
                  <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/10">
                    <div className="h-full rounded-full bg-indigo-300 transition-all" style={{ width: `${percent}%` }} />
                  </div>
                  {run.last_error && <p className="mt-2 text-sm text-red-200">{run.last_error}</p>}
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      <div className="mt-6 overflow-hidden rounded-2xl border border-white/10">
        <div className="border-b border-white/10 bg-slate-950/60 px-4 py-3 text-sm font-bold uppercase tracking-[0.2em] text-slate-300">
          Background jobs
        </div>
        {loading ? (
          <p className="px-4 py-6 text-sm text-slate-400">Loading jobs…</p>
        ) : !queue?.jobs.length ? (
          <p className="px-4 py-6 text-sm text-slate-400">No queued or running jobs.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead className="bg-slate-950/40 text-xs uppercase tracking-[0.15em] text-slate-400">
                <tr>
                  <th className="px-4 py-3 font-semibold">Kind</th>
                  <th className="px-4 py-3 font-semibold">Subject</th>
                  <th className="px-4 py-3 font-semibold">Status</th>
                  <th className="px-4 py-3 font-semibold">Run at</th>
                  <th className="px-4 py-3 font-semibold">Attempts</th>
                </tr>
              </thead>
              <tbody>
                {queue.jobs.map((job) => (
                  <tr key={job.id} className="border-t border-white/5 text-slate-200">
                    <td className="px-4 py-3">
                      <div className="font-medium text-white">{kindLabel(job.kind)}</div>
                      {job.source && <div className="text-xs text-slate-400">{job.source.replaceAll("_", " ")}</div>}
                    </td>
                    <td className="px-4 py-3">{formatSubject(job)}</td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex rounded-full border px-2 py-0.5 text-xs font-semibold ${statusBadgeClass(job.status)}`}>
                        {job.status}
                      </span>
                      {job.available_after_rate_limit && job.rate_limit_reset_at && (
                        <div className="mt-1 text-xs text-amber-200">waiting for rate limit until {new Date(job.rate_limit_reset_at).toLocaleString()}</div>
                      )}
                      {job.last_error && <div className="mt-1 text-xs text-red-200">{job.last_error}</div>}
                    </td>
                    <td className="px-4 py-3 text-slate-400">{new Date(job.run_at).toLocaleString()}</td>
                    <td className="px-4 py-3 text-slate-400">
                      {job.attempts}/{job.max_attempts}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  );
}
