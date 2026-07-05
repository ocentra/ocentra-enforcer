"""workflows/order_fulfillment.py -- a workflow calling a service."""
from app.services.order_service import OrderService


class OrderFulfillmentWorkflow:
    def __init__(self, service: OrderService):
        self.service = service

    def run(self, order_id: int):
        order = self.service.find_by_id(order_id)
        return order
