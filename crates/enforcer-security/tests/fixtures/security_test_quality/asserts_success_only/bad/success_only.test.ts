import { chargeCard } from "../src/payments";

test("charge succeeds", () => {
  const res = chargeCard({ amount: 100, token: "tok_visa" });
  expect(res.ok).toBe(true);
});
