"use client";

import { useEffect, useState } from "react";
import { apiFetch, type RepoPolicyResponse, type TrustedUser } from "../lib/api";
import { confirmAction } from "../lib/confirm";
import { showSuccessToast } from "../lib/toast";

export function AllowlistEditor({ repoId }: { repoId: string }) {
  const [users, setUsers] = useState<TrustedUser[]>([]);
  const [user, setUser] = useState("");
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<RepoPolicyResponse>(`/api/repos/${encodeURIComponent(repoId)}`)
      .then((repo) => setUsers(repo.trusted_users ?? []))
      .catch((err: Error) => setError(err.message));
  }, [repoId]);

  async function addUser() {
    if (!user.trim()) return;
    setBusy(true); setError(null);
    try {
      const added = await apiFetch<TrustedUser>(`/api/repos/${encodeURIComponent(repoId)}/allowlist`, {
        method: "POST",
        body: JSON.stringify({ user: user.trim(), reason: reason.trim() || undefined }),
      });
      setUsers((current) => [added, ...current.filter((u) => u.subject_id !== added.subject_id)]);
      setUser(""); setReason("");
      showSuccessToast(`${added.login ?? added.subject_id} was added to the allowlist.`);
    } catch (err) { setError(err instanceof Error ? err.message : "Failed to add user"); }
    finally { setBusy(false); }
  }

  async function removeUser(subjectId: string, label: string) {
    const confirmed = await confirmAction({
      title: "Remove from allowlist",
      message: `Remove ${label} from this repository's auth allowlist? They will need to verify normally again.`,
      confirmLabel: "Remove",
      cancelLabel: "Keep",
      tone: "danger",
    });
    if (!confirmed) return;
    setBusy(true); setError(null);
    try {
      await apiFetch<void>(`/api/repos/${encodeURIComponent(repoId)}/allowlist/${encodeURIComponent(subjectId)}`, { method: "DELETE" });
      setUsers((current) => current.filter((u) => u.subject_id !== subjectId));
      showSuccessToast(`${label} was removed from the allowlist.`);
    } catch (err) { setError(err instanceof Error ? err.message : "Failed to remove user"); }
    finally { setBusy(false); }
  }

  return <section className="rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
    <h2 className="text-2xl font-bold text-white">Auth allowlist</h2>
    <p className="mt-1 text-sm text-slate-400">Add or remove trusted contributors by GitHub login or numeric user id.</p>
    <div className="mt-5 grid gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-stretch">
      <input className="h-11 rounded-xl border border-white/10 bg-slate-950/80 px-4 text-sm text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-500 focus:ring-4" placeholder="login or user id" value={user} onChange={(e) => setUser(e.target.value)} />
      <input className="h-11 rounded-xl border border-white/10 bg-slate-950/80 px-4 text-sm text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-500 focus:ring-4" placeholder="reason (optional)" value={reason} onChange={(e) => setReason(e.target.value)} />
      <button className="inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold leading-none text-slate-950 shadow-lg shadow-cyan-950/30 transition hover:bg-cyan-200 disabled:opacity-60 sm:min-w-[5.5rem]" disabled={busy || !user.trim()} onClick={addUser}>Add</button>
    </div>
    {error && <p className="mt-3 text-sm font-medium text-red-300">{error}</p>}
    <ul className="mt-5 overflow-hidden rounded-2xl border border-white/10">
      {users.length === 0 && <li className="bg-slate-950/40 p-4 text-sm text-slate-400">No authorized users.</li>}
      {users.map((u) => {
        const label = u.login ?? u.subject_id;
        return <li key={u.subject_id} className="flex items-start justify-between gap-4 border-b border-white/10 bg-slate-950/40 p-4 last:border-b-0 sm:items-center">
          <div className="min-w-0"><p className="font-semibold text-white">{label}</p>{u.reason && <p className="mt-0.5 text-sm text-slate-400">{u.reason}</p>}</div>
          <button className="shrink-0 text-sm font-bold leading-none text-red-300 transition hover:text-red-200 disabled:opacity-60" disabled={busy} onClick={() => removeUser(u.subject_id, label)}>Remove</button>
        </li>;
      })}
    </ul>
  </section>;
}
