import { VerifySuccessClient } from "../../../../components/VerifySuccessClient";

export default async function VerifySuccessPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <VerifySuccessClient sessionId={sessionId} />;
}
