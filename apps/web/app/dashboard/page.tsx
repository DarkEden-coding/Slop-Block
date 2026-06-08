import Link from "next/link";
import { AuthGate, AuthPanel } from "../../components/AuthPanel";
import { ReposList } from "../../components/ReposList";

export default function DashboardPage() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-10 text-slate-100">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_15%_0%,rgba(34,211,238,0.16),transparent_30%),radial-gradient(circle_at_85%_10%,rgba(59,130,246,0.14),transparent_32%)]" />
      <div className="relative mx-auto max-w-6xl">
        <div className="mb-10 flex flex-col justify-between gap-6 border-b border-white/10 pb-8 lg:flex-row lg:items-end">
          <div>
            <p className="text-sm font-semibold uppercase tracking-[0.25em] text-cyan-300">Control center</p>
            <h1 className="mt-3 text-4xl font-black tracking-tight text-white sm:text-5xl">Dashboard</h1>
            <p className="mt-3 max-w-2xl text-slate-400">Review installed repositories and edit human-verification policies in one clean workspace.</p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <AuthPanel compact />
            <Link className="rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm font-semibold text-slate-100 shadow-lg shadow-black/20 transition hover:border-cyan-300/40 hover:bg-white/10" href="/dashboard/installations">Installations</Link>
          </div>
        </div>
        <AuthGate><ReposList /></AuthGate>
      </div>
    </main>
  );
}
