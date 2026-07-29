const transitions = {
  pending: ["shipped", "cancelled"],
  shipped: [],
  cancelled: [],
} as const;

export class OrderViewModel {
  status: keyof typeof transitions = "pending";

  ship(): void {
    this.status = assertTransition(transitions, this.status, "shipped");
  }
}

function assertTransition<T extends Record<string, readonly string[]>>(
  map: T,
  from: keyof T,
  to: string,
): keyof T {
  if (!map[from].includes(to)) {
    throw new Error(`invalid transition ${String(from)} -> ${to}`);
  }
  return to as keyof T;
}
