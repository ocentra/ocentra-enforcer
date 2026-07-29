import { authorizePayment } from "../src/payments";

test("authorize rejects a tampered signature", () => {
  expect(() => authorizePayment({ amount: 100, signature: "bad" })).toThrow();
});
