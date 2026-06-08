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

  if (loading) return <div className="rounded-lg border border-white/10 px-4 py-2 text-sm opacity-80">Checking login…</div>;

  if (!me?.authenticated) {
    return <a href={me?.login_url ?? absoluteApi("/api/auth/github/start")} className="rounded-lg bg-cyan-400 px-5 py-3 font-semibold text-slate-950">Login with GitHub</a>;
  }

  return (
    <div className={compact ? "flex items-center gap-3" : "rounded-2xl border border-white/10 bg-white/5 p-5"}>
      {me.user?.avatar_url && <Image src={me.user.avatar_url} alt="" width={36} height={36} className="h-9 w-9 rounded-full" unoptimized />}
      <div className={compact ? "text-sm" : ""}>
        <p className={compact ? "font-semibold text-slate-800" : "font-semibold text-white"}>Signed in as {me.user?.login}</p>
        {!compact && <p className="mt-1 text-sm text-slate-300">You can now configure installed repositories.</p>}
      </div>
      <button onClick={logout} className={compact ? "rounded-lg border bg-white px-3 py-2 text-sm" : "mt-4 rounded-lg border border-white/20 px-4 py-2 text-sm font-semibold text-white"}>Logout</button>
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

  if (loading) return <div className="rounded-xl border bg-white p-6 text-slate-600">Checking GitHub login…</div>;
  if (!me?.authenticated) {
    return <div className="rounded-xl border bg-white p-8 shadow-sm"><h2 className="text-2xl font-bold text-slate-950">Login required</h2><p className="mt-2 text-slate-600">Sign in with GitHub to manage installations, repositories, policies, and allowlists.</p><a href={me?.login_url ?? absoluteApi("/api/auth/github/start")} className="mt-6 inline-block rounded-lg bg-slate-950 px-5 py-3 font-semibold text-white">Login with GitHub</a></div>;
  }
  return <>{children}</>;
}
