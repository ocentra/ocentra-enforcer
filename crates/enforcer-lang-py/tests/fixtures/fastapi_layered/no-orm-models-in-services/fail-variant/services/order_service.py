"""services/order_service.py -- ORM model imported via a relative import
from a non-`app` package. The old `from app.models` substring check missed
this; a whole-segment `models` import check catches it."""
from ..models.order import Order


class OrderService:
    def build(self, order_id: int, total: int) -> Order:
        return Order(id=order_id, total=total)
