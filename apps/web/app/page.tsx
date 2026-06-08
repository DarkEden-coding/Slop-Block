import Link from "next/link";

export default function Home() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-16 text-white">
      <section className="mx-auto max-w-4xl">
        <p className="mb-4 text-sm font-semibold uppercase tracking-wide text-cyan-300">GitHub Human Auth</p>
        <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">Human verification for safer GitHub triage.</h1>
        <p className="mt-6 max-w-2xl text-lg leading-8 text-slate-300">Protect issues and pull requests from automated abuse with a self-hosted GitHub App that combines GitHub OAuth and Cloudflare Turnstile.</p>
        <div className="mt-8 flex gap-4"><Link href="/dashboard" className="rounded-lg bg-cyan-400 px-5 py-3 font-semibold text-slate-950">Open dashboard</Link><Link href="/dashboard/installations" className="rounded-lg border border-white/20 px-5 py-3 font-semibold">View installations</Link></div>
        <div className="mt-12 grid gap-4 md:grid-cols-3">{["Minimal data collection", "Repository-level policies", "Clear contributor flow"].map((t) => <div key={t} className="rounded-xl border border-white/10 bg-white/5 p-5">{t}</div>)}</div>
      </section>
    </main>
  );
}
