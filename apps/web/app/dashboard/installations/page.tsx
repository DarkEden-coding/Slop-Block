import Link from "next/link";
import { AuthGate } from "../../../components/AuthPanel";
import { InstallationsList } from "../../../components/InstallationsList";

export default function InstallationsPage() {
  return <main className="min-h-screen bg-slate-50 px-6 py-10"><div className="mx-auto max-w-5xl"><Link href="/dashboard" className="text-sm text-cyan-700">← Back to dashboard</Link><h1 className="mt-4 text-3xl font-bold text-slate-950">GitHub App installations</h1><p className="mb-8 mt-2 text-slate-600">Accounts where the app is installed. Installation data is used only to locate repositories you manage.</p><AuthGate><InstallationsList /></AuthGate></div></main>;
}
