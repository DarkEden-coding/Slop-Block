import Link from "next/link";
import { AuthPanel } from "../components/AuthPanel";

const features = ["OAuth-gated admin access", "Repository policy control", "Trusted contributor allowlists"];

export default function Home() {
  return (
    <main className="min-h-screen overflow-hidden bg-slate-950 px-6 py-16 text-white">
      <div className="pointer-events-none fixed inset-0 -z-0 bg-[radial-gradient(circle_at_20%_10%,rgba(34,211,238,0.22),transparent_34%),radial-gradient(circle_at_80%_20%,rgba(99,102,241,0.18),transparent_30%),linear-gradient(180deg,#020617_0%,#08111f_100%)]" />
      <section className="relative z-10 mx-auto max-w-6xl">
        <p className="mb-5 text-sm font-semibold uppercase tracking-[0.3em] text-cyan-300">GitHub Human Auth</p>
        <h1 className="max-w-5xl text-5xl font-black tracking-tight sm:text-7xl">
          Sleek, login-protected controls for your GitHub App.
        </h1>
        <p className="mt-7 max-w-2xl text-lg leading-8 text-slate-300">
          Sign in with GitHub to manage installed projects, verification policies, CAPTCHA/OAuth requirements, labels, comments, and trusted contributor allowlists.
        </p>

        <div className="mt-10 flex flex-wrap items-stretch gap-4">
          <AuthPanel />
          <Link href="/dashboard" className="group flex min-w-64 items-center justify-between rounded-2xl border border-cyan-300/20 bg-cyan-300/10 px-6 py-5 font-bold text-white shadow-2xl shadow-cyan-950/40 backdrop-blur transition hover:-translate-y-0.5 hover:border-cyan-200/50 hover:bg-cyan-300/15">
            Open dashboard <span className="text-cyan-200 transition group-hover:translate-x-1">→</span>
          </Link>
        </div>

        <div className="mt-16 grid gap-px overflow-hidden rounded-3xl border border-white/10 bg-white/10 shadow-2xl shadow-black/30 md:grid-cols-3">
          {features.map((title) => (
            <div key={title} className="bg-slate-950/70 p-6 backdrop-blur">
              <div className="mb-4 h-1 w-12 rounded-full bg-cyan-300 shadow-lg shadow-cyan-400/40" />
              <p className="text-lg font-semibold text-slate-100">{title}</p>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
