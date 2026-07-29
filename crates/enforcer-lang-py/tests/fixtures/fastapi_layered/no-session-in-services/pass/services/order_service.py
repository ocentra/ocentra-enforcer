"""services/order_service.py -- a service taking an injected repository."""
from app.repositories.order_repository import OrderRepository


class OrderService:
    def __init__(self, repo: OrderRepository):
        self.repo = repo

    def find_by_id(self, order_id: int):
        return self.repo.find_by_id(order_id)
