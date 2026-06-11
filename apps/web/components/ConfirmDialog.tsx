"use client";

import { useEffect, useState } from "react";
import { subscribeToConfirms, type ConfirmRequest } from "../lib/confirm";

function ConfirmIcon({ tone }: { tone: ConfirmRequest["tone"] }) {
  if (tone === "danger") {
    return (
      <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-red-400/30 bg-red-500/10 text-red-200 shadow-lg shadow-red-950/30">
        <svg aria-hidden="true" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v4m0 4h.01M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z" />
        </svg>
      </span>
    );
  }

  return (
    <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/30 bg-cyan-300/10 text-cyan-200 shadow-lg shadow-cyan-950/30">
      <svg aria-hidden="true" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
      </svg>
    </span>
  );
}

export function ConfirmDialog() {
  const [request, setRequest] = useState<ConfirmRequest | null>(null);

  useEffect(() => {
    return subscribeToConfirms((next) => {
      setRequest((current) => {
        current?.resolve(false);
        return next;
      });
    });
  }, []);

  useEffect(() => {
    if (!request) return;

    const activeRequest = request;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        activeRequest.resolve(false);
        setRequest(null);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [request]);

  if (!request) return null;

  const activeRequest = request;
  const confirmButtonClass =
    activeRequest.tone === "danger"
      ? "bg-red-400 text-red-950 shadow-red-950/30 hover:bg-red-300"
      : "bg-cyan-300 text-slate-950 shadow-cyan-950/30 hover:bg-cyan-200";

  function close(confirmed: boolean) {
    activeRequest.resolve(confirmed);
    setRequest(null);
  }

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="Dismiss dialog"
        className="absolute inset-0 bg-slate-950/75 backdrop-blur-sm"
        onClick={() => close(false)}
      />
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
        aria-describedby="confirm-dialog-message"
        className="animate-modal-in relative w-full max-w-md rounded-3xl border border-white/10 bg-slate-950/95 p-6 shadow-2xl shadow-black/50 backdrop-blur"
      >
        <div className="flex items-start gap-4">
          <ConfirmIcon tone={activeRequest.tone} />
          <div className="min-w-0 flex-1">
            <h2 id="confirm-dialog-title" className="text-xl font-bold text-white">
              {activeRequest.title}
            </h2>
            <p id="confirm-dialog-message" className="mt-2 text-sm leading-relaxed text-slate-300">
              {activeRequest.message}
            </p>
          </div>
        </div>
        <div className="mt-6 flex flex-col-reverse gap-3 sm:flex-row sm:justify-end">
          <button
            type="button"
            className="inline-flex h-11 items-center justify-center rounded-xl border border-white/10 px-5 text-sm font-semibold text-slate-200 transition hover:bg-white/10"
            onClick={() => close(false)}
          >
            {activeRequest.cancelLabel}
          </button>
          <button
            type="button"
            autoFocus
            className={`inline-flex h-11 items-center justify-center rounded-xl px-5 text-sm font-bold shadow-xl transition hover:-translate-y-0.5 ${confirmButtonClass}`}
            onClick={() => close(true)}
          >
            {activeRequest.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
