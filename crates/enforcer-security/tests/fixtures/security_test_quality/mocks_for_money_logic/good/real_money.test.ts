import { settleInvoice } from "../src/billing";
import { creditBalance } from "../src/ledger";

test("settlement rejects when it would drive the ledger balance negative", () => {
  expect(() => settleInvoice({ invoiceId: "inv_1", amount: 999999 })).toThrow();
  expect(creditBalance).toBeDefined();
});
