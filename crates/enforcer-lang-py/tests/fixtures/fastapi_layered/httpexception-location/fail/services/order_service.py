"""services/order_service.py -- raising HTTPException outside routers/."""
from fastapi import HTTPException

from app.repositories.order_repository import OrderRepository


class OrderService:
    def __init__(self, repo: OrderRepository):
        self.repo = repo

    def find_by_id(self, order_id: int):
        order = self.repo.find_by_id(order_id)
        if order is None:
            raise HTTPException(status_code=404, detail="order not found")
        return order
