import Link from "next/link";
import { AllowlistEditor } from "../../../../components/AllowlistEditor";
import { AuthGate } from "../../../../components/AuthPanel";
import { RepoPolicyEditor } from "../../../../components/RepoPolicyEditor";
import { apiFetch, type RepoPolicyResponse, type TrustedUser } from "../../../../lib/api";

export default async function RepoPage({ params }: { params: Promise<{ repoId: string }> }) {
  const { repoId } = await params;
  let trustedUsers: TrustedUser[] = [];
  try {
    const repo = await apiFetch<RepoPolicyResponse>(`/api/repos/${encodeURIComponent(repoId)}`);
    trustedUsers = repo.trusted_users ?? [];
  } catch {}

  return (
    <main className="min-h-screen bg-slate-950 px-6 py-10 text-slate-100">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_18%_0%,rgba(34,211,238,0.16),transparent_30%),radial-gradient(circle_at_80%_8%,rgba(99,102,241,0.12),transparent_30%)]" />
      <div className="relative mx-auto max-w-6xl">
        <Link href="/dashboard" className="inline-flex items-center text-sm font-semibold leading-none text-cyan-300 transition hover:text-cyan-100">← Back to dashboard</Link>
        <h1 className="mt-5 text-4xl font-black tracking-tight text-white sm:text-5xl">Repository policy</h1>
        <p className="mb-8 mt-3 max-w-3xl text-slate-400">Configure when contributors must pass OAuth and CAPTCHA checks for repository #{repoId}.</p>
        <AuthGate>
          <div className="space-y-6">
            <RepoPolicyEditor repoId={repoId} />
            <AllowlistEditor repoId={repoId} initialUsers={trustedUsers} />
          </div>
        </AuthGate>
      </div>
    </main>
  );
}
