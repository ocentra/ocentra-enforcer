import fc from "fast-check";
import { computeExchangeRate } from "../src/pricing";

test("exchange rate fuzz", () => {
  fc.assert(
    fc.property(fc.integer(), (amount) => {
      const rate = computeExchangeRate(amount);
      expect(rate).toBeGreaterThanOrEqual(0);
    })
  );
});
