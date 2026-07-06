// FRONT-02 fail fixture: a useEffect performing a raw fetch for
// data-loading instead of a query hook.
import { useEffect, useState } from "react";

export function OrderList() {
  const [orders, setOrders] = useState([]);
  useEffect(() => {
    fetch("/api/orders")
      .then((res) => res.json())
      .then(setOrders);
  }, []);
  return <ul>{orders.length}</ul>;
}
