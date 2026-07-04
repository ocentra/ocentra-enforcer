// good: halt-all, atomic, authed, audited, replay-safe, and tested.
// kill-switch-tested: test_haltAllPayments_full
export function haltAllPayments(req: Request) {
  requireAuth(req);
  withLock('payments-halt', () => {
    setPaymentsHalted(true);
  });
  auditLog('payments-halted', { idempotencyKey: req.body.idempotencyKey });
}

export const killSwitch = haltAllPayments;
