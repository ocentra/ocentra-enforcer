import { cancelOrder } from "./orders";

test("test_cancel_order_already_shipped_raises", () => {
    expect(() => cancelOrder("shipped")).toThrow(message="order already shipped");
});
