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

  async function removeUser(subjectId: string) {
    setBusy(true); setError(null);
    try {
      await apiFetch<void>(`/api/repos/${encodeURIComponent(repoId)}/allowlist/${encodeURIComponent(subjectId)}`, { method: "DELETE" });
      setUsers((current) => current.filter((u) => u.subject_id !== subjectId));
    } catch (err) { setError(err instanceof Error ? err.message : "Failed to remove user"); }
    finally { setBusy(false); }
  }

  return <section className="mt-6 rounded-2xl border bg-white p-6 shadow-sm">
    <h2 className="text-xl font-bold text-slate-950">Manual allowlist</h2>
    <p className="mt-1 text-sm text-slate-600">Add trusted contributors by GitHub login or numeric user id.</p>
    <div className="mt-4 grid gap-3 sm:grid-cols-[1fr_1fr_auto]">
      <input className="rounded-md border px-3 py-2" placeholder="login or user id" value={user} onChange={(e) => setUser(e.target.value)} />
      <input className="rounded-md border px-3 py-2" placeholder="reason (optional)" value={reason} onChange={(e) => setReason(e.target.value)} />
      <button className="rounded-lg bg-cyan-700 px-4 py-2 font-semibold text-white disabled:opacity-60" disabled={busy || !user.trim()} onClick={addUser}>Add</button>
    </div>
    {error && <p className="mt-3 text-sm text-red-700">{error}</p>}
    <ul className="mt-5 divide-y rounded-lg border">
      {users.length === 0 && <li className="p-4 text-sm text-slate-500">No manually allowlisted users.</li>}
      {users.map((u) => <li key={u.subject_id} className="flex items-center justify-between gap-3 p-4">
        <div><p className="font-medium text-slate-900">{u.login ?? u.subject_id}</p>{u.reason && <p className="text-sm text-slate-500">{u.reason}</p>}</div>
        <button className="text-sm font-semibold text-red-700 disabled:opacity-60" disabled={busy} onClick={() => removeUser(u.subject_id)}>Remove</button>
      </li>)}
    </ul>
  </section>;
}
