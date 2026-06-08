"use client";

import Image from "next/image";
import type React from "react";
import { useEffect, useState } from "react";
import { API_BASE_URL, apiFetch, type AuthMe } from "../lib/api";

function absoluteApi(path: string) {
  if (!API_BASE_URL) return path;
  return `${API_BASE_URL.replace(/\/$/, "")}${path}`;
}

export function AuthPanel({ compact = false }: { compact?: boolean }) {
  const [me, setMe] = useState<AuthMe | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<AuthMe>("/api/auth/me")
      .then(setMe)
      .catch(() => setMe({ authenticated: false, user: null, login_url: absoluteApi("/api/auth/github/start") }))
      .finally(() => setLoading(false));
  }, []);

  async function logout() {
    await apiFetch<void>("/api/auth/logout", { method: "POST" });
    window.location.href = "/";
  }

  if (loading) return <div className="rounded-xl border border-white/10 bg-white/5 px-4 py-2 text-sm text-slate-300 shadow-lg shadow-black/20">Checking login…</div>;

  if (!me?.authenticated) {
    return <a href={me?.login_url ?? absoluteApi("/api/auth/github/start")} className="rounded-xl bg-cyan-300 px-5 py-3 font-bold text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200">Login with GitHub</a>;
  }

  return (
    <div className={compact ? "flex items-center gap-3 rounded-2xl border border-white/10 bg-white/5 px-3 py-2 shadow-xl shadow-black/20 backdrop-blur" : "min-w-80 rounded-2xl border border-white/10 bg-white/[0.06] p-5 shadow-2xl shadow-black/30 backdrop-blur"}>
      {me.user?.avatar_url && <Image src={me.user.avatar_url} alt="" width={40} height={40} className="h-10 w-10 rounded-full ring-2 ring-cyan-300/30" unoptimized />}
      <div className="min-w-0">
        <p className="truncate font-semibold text-white">Signed in as {me.user?.login}</p>
        {!compact && <p className="mt-1 text-sm text-slate-400">You can now configure installed repositories.</p>}
      </div>
      <button onClick={logout} className={compact ? "rounded-lg border border-white/10 px-3 py-2 text-sm font-semibold text-slate-200 transition hover:bg-white/10" : "mt-5 rounded-lg border border-white/15 px-4 py-2 text-sm font-semibold text-white transition hover:border-cyan-300/40 hover:bg-white/10"}>Logout</button>
    </div>
  );
}

export function AuthGate({ children }: { children: React.ReactNode }) {
  const [me, setMe] = useState<AuthMe | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<AuthMe>("/api/auth/me")
      .then(setMe)
      .catch(() => setMe({ authenticated: false, user: null, login_url: absoluteApi("/api/auth/github/start") }))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div className="rounded-2xl border border-white/10 bg-white/5 p-6 text-slate-300 shadow-xl shadow-black/20">Checking GitHub login…</div>;
  if (!me?.authenticated) {
    return <div className="rounded-3xl border border-white/10 bg-white/[0.06] p-8 shadow-2xl shadow-black/30 backdrop-blur"><h2 className="text-2xl font-bold text-white">Login required</h2><p className="mt-2 text-slate-400">Sign in with GitHub to manage installations, repositories, policies, and allowlists.</p><a href={me?.login_url ?? absoluteApi("/api/auth/github/start")} className="mt-6 inline-block rounded-xl bg-cyan-300 px-5 py-3 font-bold text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:bg-cyan-200">Login with GitHub</a></div>;
  }
  return <>{children}</>;
}
