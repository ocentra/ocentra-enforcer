import { formatCurrency } from "@/lib/format";
import { Card } from "@/components/Card";

export function CheckoutSummary({ total }: { total: number }) {
  return <Card>{formatCurrency(total)}</Card>;
}
