// FRONT-01 pass fixture: a router-layer module delegating to a service,
// with no repository symbol referenced at all.
import { OrderService } from "../services/order-service";

export function createOrder(req: Request): Response {
  const service = new OrderService();
  const order = service.create(req.body);
  return Response.json(order);
}
