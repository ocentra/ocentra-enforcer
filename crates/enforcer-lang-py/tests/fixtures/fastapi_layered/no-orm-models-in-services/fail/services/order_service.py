"""services/order_service.py -- importing an ORM model class directly."""
from app.models.order import Order


class OrderService:
    def build(self, order_id: int, total: int) -> Order:
        return Order(id=order_id, total=total)
