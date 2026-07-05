"""domain/order.py -- pure domain, no FastAPI/HTTP import or exception."""
from app.domain.errors import InvalidDiscountError


class Order:
    def __init__(self, order_id: int, total: int):
        self.order_id = order_id
        self.total = total

    def apply_discount(self, percent: int):
        if percent > 100:
            raise InvalidDiscountError("invalid discount")
        self.total = self.total - (self.total * percent // 100)
