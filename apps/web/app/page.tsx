import Link from "next/link";
import { AuthPanel } from "../components/AuthPanel";

export default function Home() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-16 text-white">
      <section className="mx-auto max-w-4xl">
        <p className="mb-4 text-sm font-semibold uppercase tracking-wide text-cyan-300">GitHub Human Auth</p>
        <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">Login-protected configuration for your GitHub App.</h1>
        <p className="mt-6 max-w-2xl text-lg leading-8 text-slate-300">Sign in with GitHub to manage installed projects, repository policies, CAPTCHA/OAuth requirements, labels, comments, and trusted contributor allowlists.</p>
        <div className="mt-8 flex flex-wrap gap-4"><AuthPanel /><Link href="/dashboard" className="rounded-lg border border-white/20 px-5 py-3 font-semibold">Open dashboard</Link></div>
        <div className="mt-12 grid gap-4 md:grid-cols-3">{["GitHub OAuth admin login", "Repository-level settings", "Contributor allowlists"].map((t) => <div key={t} className="rounded-xl border border-white/10 bg-white/5 p-5">{t}</div>)}</div>
      </section>
    </main>
  );
}
