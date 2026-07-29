// good: retry is bounded by maxRetries and each attempt charges the
// caller — attacker-cost stays >= system-cost.
export async function settleWithRetry(request: SettleRequest) {
  const maxRetries = 3;
  retry(() => {
    chargeCaller(request.callerId, request.attemptFee);
    callPaymentGateway(request);
  });
}
