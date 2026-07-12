import { processRefund } from "../src/refunds";

test("refund does not crash", () => {
  expect(() => processRefund({ orderId: "ord_1", amount: 50 })).not.toThrow();
});
