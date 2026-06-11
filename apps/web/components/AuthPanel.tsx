"use client";

import Image from "next/image";
import type React from "react";
import { useEffect, useState } from "react";
import { API_BASE_URL, apiFetch, type AuthMe } from "../lib/api";
import { showSuccessToast } from "../lib/toast";

function absoluteApi(path: string) {
  if (!API_BASE_URL) return path;
  return `${API_BASE_URL.replace(/\/$/, "")}${path}`;
}

function withReturnTo(loginUrl: string) {
  if (typeof window === "undefined") return loginUrl;
  const url = new URL(loginUrl, window.location.origin);
  url.searchParams.set("return_to", window.location.pathname + window.location.search);
  return url.toString();
}

export function AuthPanel({ compact = false, hideLogin = false }: { compact?: boolean; hideLogin?: boolean }) {
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
    showSuccessToast("You have been signed out.");
    window.setTimeout(() => {
      window.location.href = "/";
    }, 450);
  }

  if (loading) {
    if (hideLogin) return null;
    return (
      <div className="inline-flex h-11 items-center rounded-xl border border-white/10 bg-white/5 px-4 text-sm leading-none text-slate-300 shadow-lg shadow-black/20">
        Checking login…
      </div>
    );
  }

  if (!me?.authenticated) {
    if (hideLogin) return null;
    return (
      <a
        href={withReturnTo(me?.login_url ?? absoluteApi("/api/auth/github/start"))}
        className="inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold leading-none text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:-translate-y-0.5 hover:bg-cyan-200"
      >
        Login with GitHub
      </a>
    );
  }

  return (
    <div
      className={
        compact
          ? "inline-flex h-11 max-w-full items-center gap-3 rounded-xl border border-white/10 bg-white/5 px-3 shadow-xl shadow-black/20 backdrop-blur"
          : "min-w-80 rounded-2xl border border-white/10 bg-white/[0.06] p-5 shadow-2xl shadow-black/30 backdrop-blur"
      }
    >
      {me.user?.avatar_url && (
        <Image src={me.user.avatar_url} alt="" width={32} height={32} className="h-8 w-8 shrink-0 rounded-full ring-2 ring-cyan-300/30" unoptimized />
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-semibold leading-tight text-white">Signed in as {me.user?.login}</p>
        {!compact && <p className="mt-1 text-sm text-slate-400">You can now configure installed repositories.</p>}
      </div>
      <button
        onClick={logout}
        className={
          compact
            ? "inline-flex h-8 shrink-0 items-center justify-center rounded-lg border border-white/10 px-3 text-sm font-semibold leading-none text-slate-200 transition hover:bg-white/10"
            : "mt-4 inline-flex items-center justify-center self-start rounded-lg border border-white/15 px-4 py-2 text-sm font-semibold text-white transition hover:border-cyan-300/40 hover:bg-white/10"
        }
      >
        Logout
      </button>
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
    return (
      <div className="rounded-3xl border border-white/10 bg-white/[0.06] p-8 shadow-2xl shadow-black/30 backdrop-blur">
        <h2 className="text-2xl font-bold text-white">Login required</h2>
        <p className="mt-2 max-w-xl text-slate-400">Sign in with GitHub to manage installations, repositories, policies, and allowlists.</p>
        <a
          href={withReturnTo(me?.login_url ?? absoluteApi("/api/auth/github/start"))}
          className="mt-6 inline-flex h-11 items-center justify-center rounded-xl bg-cyan-300 px-5 text-sm font-bold leading-none text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:bg-cyan-200"
        >
          Login with GitHub
        </a>
      </div>
    );
  }
  return <>{children}</>;
}
