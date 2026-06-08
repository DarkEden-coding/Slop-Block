import Link from "next/link";
import { AuthGate, AuthPanel } from "../../components/AuthPanel";
import { ReposList } from "../../components/ReposList";

export default function DashboardPage() {
  return <main className="min-h-screen bg-slate-50 px-6 py-10"><div className="mx-auto max-w-5xl"><div className="mb-8 flex items-center justify-between gap-4"><div><h1 className="text-3xl font-bold text-slate-950">Dashboard</h1><p className="mt-2 text-slate-600">Review installed repositories and edit verification policies.</p></div><div className="flex items-center gap-3"><AuthPanel compact /><Link className="rounded-lg border bg-white px-4 py-2" href="/dashboard/installations">Installations</Link></div></div><AuthGate><ReposList /></AuthGate></div></main>;
}
