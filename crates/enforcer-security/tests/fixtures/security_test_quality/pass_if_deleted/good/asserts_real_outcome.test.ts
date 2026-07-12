import { chargeCard } from "../src/payments";

test("charge is processed for exactly the requested amount", () => {
  const res = chargeCard({ amount: 100, token: "tok_visa" });
  expect(res.amountCharged).toBe(100);
});
