"use client";

import { useEffect, useState } from "react";
import { subscribeToToasts, type Toast } from "../lib/toast";

const TOAST_TTL_MS = 6000;

const toneStyles: Record<Toast["tone"], string> = {
  error: "border-red-400/40 bg-red-950/90 text-red-100 shadow-red-950/40",
  success: "border-emerald-400/40 bg-emerald-950/90 text-emerald-100 shadow-emerald-950/40",
  info: "border-cyan-300/40 bg-slate-950/90 text-cyan-100 shadow-cyan-950/40",
};

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
    <div className="pointer-events-none fixed bottom-6 right-6 z-50 flex w-full max-w-sm flex-col gap-3">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role="alert"
          className={`pointer-events-auto flex items-start justify-between gap-3 rounded-2xl border p-4 shadow-2xl backdrop-blur ${toneStyles[toast.tone]}`}
        >
          <div className="min-w-0">
            {toast.title && <p className="font-bold">{toast.title}</p>}
            <p className="break-words text-sm leading-relaxed">{toast.message}</p>
          </div>
          <button
            aria-label="Dismiss notification"
            className="shrink-0 text-lg font-bold leading-none opacity-60 transition hover:opacity-100"
            onClick={() => setToasts((current) => current.filter((t) => t.id !== toast.id))}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
