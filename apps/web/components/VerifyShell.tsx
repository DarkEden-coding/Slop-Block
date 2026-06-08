import type { ReactNode } from "react";

export function VerifyShell({
  eyebrow = "Human verification",
  title,
  description,
  children,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <main className="min-h-screen overflow-hidden bg-slate-950 px-6 py-12 text-slate-100 sm:py-16">
      <div className="pointer-events-none fixed inset-0 -z-0 bg-[radial-gradient(circle_at_20%_10%,rgba(34,211,238,0.18),transparent_34%),radial-gradient(circle_at_80%_20%,rgba(99,102,241,0.14),transparent_30%),linear-gradient(180deg,#020617_0%,#08111f_100%)]" />
      <div className="relative z-10 mx-auto max-w-2xl">
        <p className="text-sm font-semibold uppercase tracking-[0.25em] text-cyan-300">{eyebrow}</p>
        <h1 className="mt-4 text-4xl font-black tracking-tight text-white sm:text-5xl">{title}</h1>
        {description ? <p className="mt-4 max-w-xl text-base leading-7 text-slate-400">{description}</p> : null}
        <div className="mt-8 rounded-3xl border border-white/10 bg-slate-950/70 p-6 shadow-2xl shadow-black/30 backdrop-blur sm:p-8">
          {children}
        </div>
      </div>
    </main>
  );
}

export function VerifyStepIndicator({
  current,
  oauthDone,
}: {
  current: "oauth" | "captcha" | "done";
  oauthDone: boolean;
}) {
  const steps = [
    { id: "oauth", label: "Sign in with GitHub" },
    { id: "captcha", label: "Complete CAPTCHA" },
    { id: "done", label: "Return to GitHub" },
  ] as const;

  return (
    <ol className="mb-8 grid gap-3 sm:grid-cols-3">
      {steps.map((step, index) => {
        const stepIndex = steps.findIndex((item) => item.id === current);
        const active = step.id === current;
        const complete =
          (step.id === "oauth" && oauthDone) ||
          (step.id === "captcha" && current === "done") ||
          index < stepIndex;
        return (
          <li
            key={step.id}
            className={`rounded-2xl border px-4 py-3 text-sm ${
              active
                ? "border-cyan-300/40 bg-cyan-300/10 text-white"
                : complete
                  ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-100"
                  : "border-white/10 bg-white/5 text-slate-400"
            }`}
          >
            <span className="mb-1 block text-xs font-semibold uppercase tracking-[0.2em] opacity-80">
              Step {index + 1}
            </span>
            <span className="font-semibold">{step.label}</span>
          </li>
        );
      })}
    </ol>
  );
}
