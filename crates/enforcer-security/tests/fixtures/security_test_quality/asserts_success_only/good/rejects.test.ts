import { chargeCard } from "../src/payments";

test("charge rejects an invalid token", () => {
  expect(() => chargeCard({ amount: 100, token: "invalid" })).toThrow();
});

test("charge succeeds with a valid token", () => {
  const res = chargeCard({ amount: 100, token: "tok_visa" });
  expect(res.ok).toBe(true);
});
