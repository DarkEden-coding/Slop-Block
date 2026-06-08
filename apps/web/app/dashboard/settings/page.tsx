import Link from "next/link";
import { AuthGate } from "../../../components/AuthPanel";
import { CaptchaSettingsEditor } from "../../../components/CaptchaSettingsEditor";

export default function CaptchaSettingsPage() {
  return (
    <main className="min-h-screen bg-slate-950 px-6 py-10 text-slate-100">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_12%_0%,rgba(34,211,238,0.16),transparent_30%),radial-gradient(circle_at_88%_12%,rgba(59,130,246,0.14),transparent_32%)]" />
      <div className="relative mx-auto max-w-4xl">
        <Link href="/dashboard" className="inline-flex items-center text-sm font-semibold leading-none text-cyan-300 transition hover:text-cyan-100">
          ← Back to dashboard
        </Link>
        <h1 className="mt-5 text-4xl font-black tracking-tight text-white sm:text-5xl">CAPTCHA settings</h1>
        <p className="mb-8 mt-3 max-w-3xl text-slate-400">
          Enable one or more CAPTCHA providers for contributor verification. Repositories can optionally override the installation default.
        </p>
        <AuthGate>
          <CaptchaSettingsEditor />
        </AuthGate>
      </div>
    </main>
  );
}
