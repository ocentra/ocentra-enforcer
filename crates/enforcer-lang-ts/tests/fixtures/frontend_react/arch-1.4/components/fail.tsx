import { useOrderStatus } from "@/features/orders/hooks";

export function OrderBadge() {
  const status = useOrderStatus();
  return <span>{status}</span>;
}
