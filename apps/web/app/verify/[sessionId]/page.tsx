import { VerifyClient } from "../../../components/VerifyClient";

export default async function VerifyPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <main className="min-h-screen bg-slate-50 px-6 py-10"><div className="mx-auto max-w-2xl"><VerifyClient sessionId={sessionId} /></div></main>;
}
