// FRONT-05 fail fixture: constructor injection of a concrete, `new`-able
// class instead of a symbol/interface token.
import { inject, injectable } from "tsyringe";
import { OrderService } from "./order-service";

@injectable()
export class OrderController {
  constructor(@inject(OrderService) private readonly orderService: OrderService) {}
}
