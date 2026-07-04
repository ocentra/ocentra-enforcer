// good: server-side time only, explicit skew tolerance, expiry fails
// closed on any unparseable/missing input.
const SKEW_TOLERANCE_MS = 5000;

export function isExpired(expiresAtMs: number | undefined): boolean {
  if (expiresAtMs === undefined || Number.isNaN(expiresAtMs)) {
    return true;
  }
  const now = serverNow();
  return now > expiresAtMs + SKEW_TOLERANCE_MS;
}
