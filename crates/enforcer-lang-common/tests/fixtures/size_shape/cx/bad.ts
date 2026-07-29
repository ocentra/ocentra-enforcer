export function classify(order: Order): string {
  if (order.region === "us") {
    if (order.total > 100) {
      if (order.customer.tier === "gold") {
        if (order.items.length > 3) {
          if (order.rush) {
            return "us-gold-bulk-rush";
          } else if (order.express) {
            return "us-gold-bulk-express";
          }
        } else if (order.items.length > 1) {
          return "us-gold-small";
        }
      } else if (order.customer.tier === "silver") {
        return "us-silver";
      }
    } else if (order.total > 50) {
      return "us-mid";
    }
  } else if (order.region === "eu") {
    return "eu";
  } else if (order.region === "apac") {
    return "apac";
  }
  return "default";
}
