"""services/order_service.py -- a repository constructed inline, not injected."""
from app.repositories.order_repository import OrderRepository


class OrderService:
    def find_by_id(self, order_id: int):
        repo = OrderRepository()
        return repo.find_by_id(order_id)
