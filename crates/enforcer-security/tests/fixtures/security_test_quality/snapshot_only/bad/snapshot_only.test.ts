import { renderInvoiceSummary } from "../src/invoices";

test("invoice summary matches snapshot", () => {
  const summary = renderInvoiceSummary({ invoiceId: "inv_1" });
  expect(summary).toMatchSnapshot();
});
