"use client";

import { useEffect, useState } from "react";
import { subscribeToToasts, type Toast } from "../lib/toast";

const TOAST_TTL_MS = 6000;

const toneStyles: Record<
  Toast["tone"],
  { panel: string; icon: string; accent: string }
> = {
  error: {
    panel: "border-red-400/25 bg-slate-950/95 text-red-50 shadow-red-950/40",
    icon: "border-red-400/30 bg-red-500/15 text-red-200",
    accent: "bg-red-400",
  },
  success: {
    panel: "border-emerald-400/25 bg-slate-950/95 text-emerald-50 shadow-emerald-950/40",
    icon: "border-emerald-400/30 bg-emerald-500/15 text-emerald-200",
    accent: "bg-emerald-400",
  },
  info: {
    panel: "border-cyan-300/25 bg-slate-950/95 text-cyan-50 shadow-cyan-950/40",
    icon: "border-cyan-300/30 bg-cyan-300/10 text-cyan-200",
    accent: "bg-cyan-300",
  },
};

function ToastIcon({ tone }: { tone: Toast["tone"] }) {
  if (tone === "success") {
    return (
      <svg aria-hidden="true" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
      </svg>
    );
  }

  if (tone === "error") {
    return (
      <svg aria-hidden="true" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
      </svg>
    );
  }

  return (
    <svg aria-hidden="true" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" />
    </svg>
  );
}

export function Toaster() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    return subscribeToToasts((toast) => {
      setToasts((current) => [...current, toast]);
      window.setTimeout(() => {
        setToasts((current) => current.filter((t) => t.id !== toast.id));
      }, TOAST_TTL_MS);
    });
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div
      aria-live="polite"
      aria-relevant="additions"
      className="pointer-events-none fixed bottom-6 right-6 z-50 flex w-full max-w-sm flex-col gap-3"
    >
      {toasts.map((toast) => {
        const styles = toneStyles[toast.tone];
        return (
          <div
            key={toast.id}
            role="status"
            className={`animate-toast-in pointer-events-auto relative overflow-hidden rounded-2xl border p-4 shadow-2xl backdrop-blur ${styles.panel}`}
          >
            <span className={`absolute inset-y-0 left-0 w-1 ${styles.accent}`} aria-hidden="true" />
            <div className="flex items-start gap-3 pl-2">
              <span className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border ${styles.icon}`}>
                <ToastIcon tone={toast.tone} />
              </span>
              <div className="min-w-0 flex-1 pt-0.5">
                {toast.title && <p className="font-bold leading-tight text-white">{toast.title}</p>}
                <p className={`break-words text-sm leading-relaxed ${toast.title ? "mt-1 text-slate-300" : "text-slate-100"}`}>
                  {toast.message}
                </p>
              </div>
              <button
                aria-label="Dismiss notification"
                className="shrink-0 rounded-lg px-1 text-lg font-bold leading-none text-white/50 transition hover:bg-white/10 hover:text-white"
                onClick={() => setToasts((current) => current.filter((t) => t.id !== toast.id))}
              >
                ×
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
