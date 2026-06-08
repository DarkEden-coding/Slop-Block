import Link from "next/link";
import { AuthGate } from "../../../components/AuthPanel";
import { InstallationsList } from "../../../components/InstallationsList";

export default function InstallationsPage() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-10 text-slate-100">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_20%_0%,rgba(34,211,238,0.15),transparent_32%)]" />
      <div className="relative mx-auto max-w-5xl">
        <Link href="/dashboard" className="text-sm font-semibold text-cyan-300 transition hover:text-cyan-100">← Back to dashboard</Link>
        <h1 className="mt-5 text-4xl font-black tracking-tight text-white">GitHub App installations</h1>
        <p className="mb-8 mt-3 max-w-2xl text-slate-400">Accounts where the app is installed. Installation data is used only to locate repositories you manage.</p>
        <AuthGate><InstallationsList /></AuthGate>
      </div>
    </main>
  );
}
