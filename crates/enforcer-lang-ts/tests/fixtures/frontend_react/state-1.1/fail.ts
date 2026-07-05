import { create } from "zustand";

export const useOrderStore = create((set) => ({
  orders: [],
  loadOrders: async () => {
    const response = await fetch("/api/orders");
    const data = await response.json();
    set({ orders: data });
  },
}));
