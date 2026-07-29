import { chargeCard } from "../src/payments";

test("charge is processed", () => {
  const spy = jest.fn();
  chargeCard({ amount: 100, token: "tok_visa" }, spy);
  expect(spy).toHaveBeenCalled();
});
