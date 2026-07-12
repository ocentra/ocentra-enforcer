import { processRefund } from "../src/refunds";

test("refund throws InvalidAmountError for a negative amount", () => {
  expect(() => processRefund({ orderId: "ord_1", amount: -50 })).toThrow(/InvalidAmountError/);
});
