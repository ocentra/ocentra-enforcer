"""workflows/order_fulfillment.py -- a workflow using a repository directly."""
from app.repositories.order_repository import OrderRepository


class OrderFulfillmentWorkflow:
    def __init__(self, repo: OrderRepository):
        self.repo = repo

    def run(self, order_id: int):
        order = self.repo.find_by_id(order_id)
        return order
