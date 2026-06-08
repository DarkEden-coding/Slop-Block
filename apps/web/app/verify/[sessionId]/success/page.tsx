import Link from "next/link";

export default async function VerifySuccessPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <main className="min-h-screen bg-green-50 px-6 py-16"><div className="mx-auto max-w-2xl rounded-2xl border border-green-200 bg-white p-8"><h1 className="text-3xl font-bold text-green-900">Verification complete</h1><p className="mt-4 text-green-800">Session {sessionId} is verified. You can return to GitHub; the app will update your issue or pull request shortly.</p><Link className="mt-6 inline-block text-green-700 underline" href="/">Back home</Link></div></main>;
}
