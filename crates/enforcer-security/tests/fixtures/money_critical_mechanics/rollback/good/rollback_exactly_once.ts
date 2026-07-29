// good: idempotent, atomic, and tested — safe to retry without
// duplicating the compensating effect.
// rollback-tested: test_rollbackPayment_exactly_once
export function rollbackPayment(paymentId: string) {
  if (isCompensated(paymentId)) {
    return;
  }
  withLock(paymentId, () => {
    refundToCaller(paymentId);
    compensate(paymentId);
  });
}
