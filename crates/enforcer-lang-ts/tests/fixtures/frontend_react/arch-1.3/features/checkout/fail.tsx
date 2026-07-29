import { useCartTotal } from "@/features/cart/hooks";

export function CheckoutSummary() {
  const total = useCartTotal();
  return <div>{total}</div>;
}
