"""routers/orders.py -- a router delegating persistence to a service."""
from fastapi import APIRouter, Depends

from app.services.order_service import OrderService

router = APIRouter()


@router.get("/orders/{order_id}")
def get_order(order_id: int, service: OrderService = Depends(OrderService)):
    return service.find_by_id(order_id)
