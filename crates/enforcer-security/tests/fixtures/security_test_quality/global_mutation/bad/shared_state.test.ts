import { creditBalance } from "../src/ledger";

let sharedState = { total: 0 };

test("first credit updates the shared total", () => {
  sharedState.total += creditBalance({ amount: 100 });
  expect(sharedState.total).toBe(100);
});

test("second credit builds on the shared total", () => {
  sharedState.total += creditBalance({ amount: 50 });
  expect(sharedState.total).toBe(150);
});
