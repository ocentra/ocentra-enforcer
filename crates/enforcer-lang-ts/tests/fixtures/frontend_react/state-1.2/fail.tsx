import { useEffect, useState } from "react";

export function OrderList() {
  const [orders, setOrders] = useState([]);

  useEffect(() => {
    fetch("/api/orders")
      .then((res) => res.json())
      .then((data) => setOrders(data));
  }, []);

  return <ul>{orders.map((o) => <li key={o.id}>{o.id}</li>)}</ul>;
}
