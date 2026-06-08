import { VerifyErrorClient } from "../../../../components/VerifyErrorClient";

export default async function VerifyErrorPage({ params }: { params: Promise<{ sessionId: string }> }) {
  const { sessionId } = await params;
  return <VerifyErrorClient sessionId={sessionId} />;
}
