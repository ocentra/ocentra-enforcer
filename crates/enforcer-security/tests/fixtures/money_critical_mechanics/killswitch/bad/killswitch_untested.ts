// bad: kill switch is atomic/authed/audited/replay-safe but has no
// co-located test marker — an untested kill switch is forbidden.
export function haltAllPayments(req: Request) {
  requireAuth(req);
  withLock('payments-halt', () => {
    setPaymentsHalted(true);
  });
  auditLog('payments-halted', { idempotencyKey: req.body.idempotencyKey });
}

export const killSwitch = haltAllPayments;
