import { renderInvoiceSummary } from "../src/invoices";

test("invoice summary reports the exact total due", () => {
  const summary = renderInvoiceSummary({ invoiceId: "inv_1" });
  expect(summary.totalDue).toBe(4200);
  expect(summary).toMatchSnapshot();
});
