// bad: expiry is driven by a client-supplied timestamp from the request
// body — an attacker can forge this to extend a window indefinitely.
export function isWithinWindow(req: Request, windowMs: number): boolean {
  const now = req.body.timestamp;
  return now + windowMs > req.body.expiresAt;
}
