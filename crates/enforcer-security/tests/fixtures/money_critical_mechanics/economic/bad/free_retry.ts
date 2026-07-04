// bad: unbounded retry against a backend-cost-bearing call, no charge,
// no retry bound — attacker leverage is free.
export async function settleWithRetry(request: SettleRequest) {
  retry(() => {
    callPaymentGateway(request);
  });
}
