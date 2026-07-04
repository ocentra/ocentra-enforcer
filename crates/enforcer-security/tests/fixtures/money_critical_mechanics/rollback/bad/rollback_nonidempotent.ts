// bad: rollback has no idempotency guard, no atomic wrapper, and no
// test marker — a retry can double-refund.
export function rollbackPayment(paymentId: string) {
  refundToCaller(paymentId);
  compensate(paymentId);
}
