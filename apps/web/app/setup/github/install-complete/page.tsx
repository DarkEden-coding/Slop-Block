import Link from "next/link";
import { AuthGate } from "../../../../components/AuthPanel";
import { InstallCompleteClient } from "../../../../components/InstallCompleteClient";

export default async function InstallCompletePage({ searchParams }: { searchParams: Promise<{ installation_id?: string; setup_action?: string }> }) {
  const params = await searchParams;
  const installationId = params.installation_id ?? "";
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-12 text-white">
      <div className="mx-auto max-w-4xl">
        <Link href="/" className="text-sm font-semibold text-cyan-300">← Home</Link>
        <h1 className="mt-6 text-4xl font-black tracking-tight">Finish installation setup</h1>
        <p className="mt-3 text-slate-400">Claim this installation for your dashboard account, sync repositories, then configure policies.</p>
        <AuthGate>
          <InstallCompleteClient installationId={installationId} setupAction={params.setup_action ?? ""} />
        </AuthGate>
      </div>
    </main>
  );
}
