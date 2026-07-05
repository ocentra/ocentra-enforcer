import { ApiError } from "@/lib/errors";

export async function fetchOrder(id: string) {
  const response = await fetch(`/api/orders/${id}`);
  if (!response.ok) {
    throw new ApiError("failed to load order", response.status);
  }
  return response.json();
}
