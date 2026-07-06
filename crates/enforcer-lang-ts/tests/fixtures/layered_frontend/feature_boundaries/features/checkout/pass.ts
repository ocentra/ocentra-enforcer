// FRONT-03 pass fixture: imports stay within the feature's own slice and
// `@/lib` — no deep reach into another feature's internals.
import { formatCurrency } from "@/lib/currency";
import { CheckoutSummary } from "../checkout/summary";

export function reserveInventory(orderId: string): CheckoutSummary {
  return new CheckoutSummary(orderId, formatCurrency);
}
