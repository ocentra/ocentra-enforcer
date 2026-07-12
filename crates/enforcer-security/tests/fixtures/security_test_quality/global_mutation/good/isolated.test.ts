import { creditBalance } from "../src/ledger";

let sharedState = { total: 0 };

beforeEach(() => {
  sharedState = { total: 0 };
});

test("credit updates an isolated total", () => {
  sharedState.total += creditBalance({ amount: 100 });
  expect(sharedState.total).toBe(100);
});
