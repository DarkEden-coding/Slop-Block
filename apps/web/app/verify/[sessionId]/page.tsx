import { VerifyClient } from "../../../components/VerifyClient";

export default async function VerifyPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <VerifyClient sessionId={sessionId} />;
}
