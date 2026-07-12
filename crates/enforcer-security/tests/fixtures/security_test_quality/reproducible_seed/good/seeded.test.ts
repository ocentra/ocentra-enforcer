import fc from "fast-check";
import { computeExchangeRate } from "../src/pricing";

test("exchange rate fuzz with a logged seed", () => {
  const seed = 1234567890;
  console.log(`seed=${seed}`);
  fc.assert(
    fc.property(fc.integer(), (amount) => {
      const rate = computeExchangeRate(amount);
      expect(rate).toBeGreaterThanOrEqual(0);
    }),
    { seed }
  );
});
