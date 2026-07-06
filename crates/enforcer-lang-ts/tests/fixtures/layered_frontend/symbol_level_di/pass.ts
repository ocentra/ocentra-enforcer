// FRONT-05 pass fixture: constructor injection routed through a
// symbol/interface DI token, not a concrete class.
import { inject, injectable } from "tsyringe";
import type { IOrderService } from "./order-service.types";
import { OrderServiceToken } from "./order-service.tokens";

@injectable()
export class OrderController {
  constructor(@inject(OrderServiceToken) private readonly orderService: IOrderService) {}
}
