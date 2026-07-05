"""services/order_service.py -- using a domain DTO instead of an ORM model."""
from app.domain.order_dto import OrderDto


class OrderService:
    def build(self, order_id: int, total: int) -> OrderDto:
        return OrderDto(id=order_id, total=total)
