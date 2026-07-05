import { useQuery } from "@tanstack/react-query";

export function OrderList() {
  const { data: orders = [] } = useQuery({
    queryKey: ["orders"],
    queryFn: () => fetch("/api/orders").then((res) => res.json()),
  });

  return <ul>{orders.map((o) => <li key={o.id}>{o.id}</li>)}</ul>;
}
