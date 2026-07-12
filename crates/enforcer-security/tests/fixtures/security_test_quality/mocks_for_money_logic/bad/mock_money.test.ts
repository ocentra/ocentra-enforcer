import { settleInvoice } from "../src/billing";

jest.mock("../src/ledger", () => ({
  creditBalance: jest.fn().mockReturnValue({ ok: true }),
}));

test("settlement succeeds", () => {
  const res = settleInvoice({ invoiceId: "inv_1" });
  expect(res.ok).toBe(true);
});
