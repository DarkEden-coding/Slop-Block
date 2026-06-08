"use client";

import { useState } from "react";
import { apiFetch, type TrustedUser } from "../lib/api";

export function AllowlistEditor({ repoId, initialUsers }: { repoId: string; initialUsers: TrustedUser[] }) {
  const [users, setUsers] = useState(initialUsers);
  const [user, setUser] = useState("");
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    } catch (err) { setError(err instanceof Error ? err.message : "Failed to add user"); }
    finally { setBusy(false); }
  }

  async function removeUser(subjectId: string, label: string) {
    if (!window.confirm(`Remove ${label} from this repository's auth allowlist?`)) return;
    setBusy(true); setError(null);
    try {
      await apiFetch<void>(`/api/repos/${encodeURIComponent(repoId)}/allowlist/${encodeURIComponent(subjectId)}`, { method: "DELETE" });
      setUsers((current) => current.filter((u) => u.subject_id !== subjectId));
    } catch (err) { setError(err instanceof Error ? err.message : "Failed to remove user"); }
    finally { setBusy(false); }
  }

  return <section className="mt-6 rounded-3xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-black/30 backdrop-blur">
    <h2 className="text-2xl font-bold text-white">Auth allowlist</h2>
    <p className="mt-1 text-sm text-slate-400">Add or remove trusted contributors by GitHub login or numeric user id.</p>
    <div className="mt-5 grid gap-3 sm:grid-cols-[1fr_1fr_auto]">
      <input className="rounded-xl border border-white/10 bg-slate-950/80 px-4 py-3 text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-500 focus:ring-4" placeholder="login or user id" value={user} onChange={(e) => setUser(e.target.value)} />
      <input className="rounded-xl border border-white/10 bg-slate-950/80 px-4 py-3 text-white outline-none ring-cyan-300/40 transition placeholder:text-slate-500 focus:ring-4" placeholder="reason (optional)" value={reason} onChange={(e) => setReason(e.target.value)} />
      <button className="rounded-xl bg-cyan-300 px-5 py-3 font-bold text-slate-950 shadow-lg shadow-cyan-950/30 transition hover:bg-cyan-200 disabled:opacity-60" disabled={busy || !user.trim()} onClick={addUser}>Add</button>
    </div>
    {error && <p className="mt-3 text-sm font-medium text-red-300">{error}</p>}
    <ul className="mt-5 overflow-hidden rounded-2xl border border-white/10">
      {users.length === 0 && <li className="bg-slate-950/40 p-4 text-sm text-slate-400">No authorized users.</li>}
      {users.map((u) => {
        const label = u.login ?? u.subject_id;
        return <li key={u.subject_id} className="flex items-center justify-between gap-3 border-b border-white/10 bg-slate-950/40 p-4 last:border-b-0">
          <div><p className="font-semibold text-white">{label}</p>{u.reason && <p className="text-sm text-slate-400">{u.reason}</p>}</div>
          <button className="text-sm font-bold text-red-300 transition hover:text-red-200 disabled:opacity-60" disabled={busy} onClick={() => removeUser(u.subject_id, label)}>Remove</button>
        </li>;
      })}
    </ul>
  </section>;
}
