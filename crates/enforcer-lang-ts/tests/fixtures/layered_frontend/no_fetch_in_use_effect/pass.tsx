// FRONT-02 pass fixture: data loaded via a query hook, no fetch/axios
// call inside a useEffect body.
import { useQuery } from "@tanstack/react-query";

export function OrderList() {
  const { data: orders = [] } = useQuery({
    queryKey: ["orders"],
    queryFn: fetchOrders,
  });
  return <ul>{orders.length}</ul>;
}

async function fetchOrders() {
  return [];
}
