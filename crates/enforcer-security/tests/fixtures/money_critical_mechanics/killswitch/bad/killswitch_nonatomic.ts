// bad: kill switch is authed/audited/replay-safe/tested, but the halt
// itself is not wrapped in any atomic/transactional primitive.
// kill-switch-tested: test_haltAllPayments_nonatomic
export function haltAllPayments(req: Request) {
  requireAuth(req);
  setPaymentsHalted(true);
  auditLog('payments-halted', { idempotencyKey: req.body.idempotencyKey });
}

export const killSwitch = haltAllPayments;
