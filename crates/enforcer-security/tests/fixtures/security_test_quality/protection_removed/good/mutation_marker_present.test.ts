// mutation-tested: removing the signature check makes this test fail (verified 2026-01-01)
import { authorizePayment } from "../src/payments";

test("authorize rejects a tampered signature", () => {
  expect(() => authorizePayment({ amount: 100, signature: "bad" })).toThrow();
});
