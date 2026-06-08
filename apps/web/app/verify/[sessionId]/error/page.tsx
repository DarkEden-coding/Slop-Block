import Link from "next/link";

export default async function VerifyErrorPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <main className="min-h-screen bg-red-50 px-6 py-16"><div className="mx-auto max-w-2xl rounded-2xl border border-red-200 bg-white p-8"><h1 className="text-3xl font-bold text-red-900">Verification could not be completed</h1><p className="mt-4 text-red-800">Session {sessionId} failed, expired, or was cancelled. Please retry from the GitHub comment if you still need access.</p><Link className="mt-6 inline-block text-red-700 underline" href="/">Back home</Link></div></main>;
}
