"""domain/order.py -- domain importing FastAPI/HTTP and raising HTTPException."""
from fastapi import HTTPException


class Order:
    def __init__(self, order_id: int, total: int):
        self.order_id = order_id
        self.total = total

    def apply_discount(self, percent: int):
        if percent > 100:
            raise HTTPException(status_code=400, detail="invalid discount")
        self.total = self.total - (self.total * percent // 100)
