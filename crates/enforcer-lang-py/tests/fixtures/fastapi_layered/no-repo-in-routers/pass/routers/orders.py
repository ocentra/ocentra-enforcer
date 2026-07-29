"""routers/orders.py -- a router depending on a service, not a repository.

A comment can still mention Repository by name, e.g. OrderRepository, without
tripping the check: only an actual import/annotation/call reference counts.
"""
from fastapi import APIRouter, Depends

from app.services.order_service import OrderService

router = APIRouter()


@router.get("/orders/{order_id}")
def get_order(order_id: int, service: OrderService = Depends(OrderService)):
    return service.find_by_id(order_id)
