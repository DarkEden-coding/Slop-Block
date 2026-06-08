import Link from "next/link";
import { AllowlistEditor } from "../../../../components/AllowlistEditor";
import { RepoPolicyEditor } from "../../../../components/RepoPolicyEditor";
import { apiFetch, type RepoPolicyResponse, type TrustedUser } from "../../../../lib/api";

export default async function RepoPage({ params }: { params: Promise<{ repoId: string }> }) {
  const { repoId } = await params;
  let trustedUsers: TrustedUser[] = [];
  try {
    const repo = await apiFetch<RepoPolicyResponse>(`/api/repos/${encodeURIComponent(repoId)}`);
    trustedUsers = repo.trusted_users ?? [];
  } catch {}

  return <main className="min-h-screen bg-slate-50 px-6 py-10"><div className="mx-auto max-w-3xl"><Link href="/dashboard" className="text-sm text-cyan-700">← Back to dashboard</Link><h1 className="mt-4 text-3xl font-bold text-slate-950">Repository policy</h1><p className="mb-8 mt-2 text-slate-600">Configure when contributors must pass OAuth and CAPTCHA checks for repository #{repoId}.</p><RepoPolicyEditor repoId={repoId} /><AllowlistEditor repoId={repoId} initialUsers={trustedUsers} /></div></main>;
}
