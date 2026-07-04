// bad: backend signs the client's raw, unmodified request body directly.
// No canonical reconstruction, no correlation-id log.
export function signPaymentRequest(req: Request): string {
  const sig = sign(req.body);
  return sig;
}
