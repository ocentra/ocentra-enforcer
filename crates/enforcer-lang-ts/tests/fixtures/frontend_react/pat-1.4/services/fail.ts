export async function fetchOrder(id: string) {
  const response = await fetch(`/api/orders/${id}`);
  if (!response.ok) {
    throw new Error("failed to load order");
  }
  return response.json();
}
