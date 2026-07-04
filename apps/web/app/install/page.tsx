import Link from "next/link";

const appSlug = process.env.NEXT_PUBLIC_GITHUB_APP_SLUG;

export default function InstallPage() {
  const installUrl = appSlug ? `https://github.com/apps/${appSlug}/installations/new` : null;
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-16 text-white">
      <div className="mx-auto max-w-3xl">
        <Link href="/" className="text-sm font-semibold text-cyan-300">← Home</Link>
        <h1 className="mt-6 text-5xl font-black tracking-tight">Install GitHub Human Auth</h1>
        <p className="mt-5 text-lg leading-8 text-slate-300">
          Install the hosted GitHub App on selected repositories. After GitHub redirects you back, sign in and configure repository policies.
        </p>
        {installUrl ? (
          <a className="mt-8 inline-flex rounded-xl bg-cyan-300 px-6 py-3 text-sm font-black text-slate-950 shadow-xl shadow-cyan-950/30 transition hover:bg-cyan-200" href={installUrl}>
            Install GitHub App
          </a>
        ) : (
          <p className="mt-8 rounded-2xl border border-amber-300/30 bg-amber-950/30 p-4 text-amber-100">
            NEXT_PUBLIC_GITHUB_APP_SLUG is not configured on this deployment.
          </p>
        )}
      </div>
    </main>
  );
}
