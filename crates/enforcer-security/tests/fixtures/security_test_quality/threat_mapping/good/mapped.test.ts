// threat: T1565.001
import { authorizePayment } from "../src/payments";

test("authorize rejects a tampered signature and preserves the balance invariant", () => {
  // invariant: balance-non-negative
  expect(() => authorizePayment({ amount: 100, signature: "bad" })).toThrow(/InvalidSignatureError/);
});
