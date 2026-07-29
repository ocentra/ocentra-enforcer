export function classify(order: Order): string {
  if (order.region !== "us") {
    return order.region;
  }
  if (order.total <= 50) {
    return "us-low";
  }
  if (order.customer.tier !== "gold") {
    return "us-silver";
  }
  if (order.items.length <= 1) {
    return "us-gold-small";
  }
  if (order.rush) {
    return "us-gold-bulk-rush";
  }
  return "us-gold-bulk";
}
