// FRONT-01 fail fixture: a router-layer module directly instantiating a
// repository instead of delegating to a service.
import { OrderRepository } from "../repositories/order-repository";

export function createOrder(req: Request): Response {
  const repo = new OrderRepository();
  const order = repo.insert(req.body);
  return Response.json(order);
}
