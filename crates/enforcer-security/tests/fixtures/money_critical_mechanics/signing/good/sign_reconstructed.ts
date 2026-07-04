// good: backend rebuilds the payload from trusted request context,
// canonically serializes it, logs a correlation id, then signs.
export function signPaymentRequest(req: Request, correlationId: string): string {
  const rebuilt = reconstructPayload(req.context);
  const canonical = canonicalize(rebuilt);
  logger.info('signing payment', { correlationId });
  const sig = sign(canonical);
  return sig;
}
