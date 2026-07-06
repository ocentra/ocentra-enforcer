// FRONT-03 exemption fixture: imports its OWN feature via the `@/features/`
// alias path — must stay clean because the exemption is keyed on the
// importer's path matching the imported feature name, not on the mere
// absence of an `@/features/` import.
import { CheckoutTotals } from "@/features/checkout/totals";

export function summarize(orderId: string): CheckoutTotals {
  return new CheckoutTotals(orderId);
}
