"""routers/orders.py -- a router referencing a *Repository symbol directly."""
from fastapi import APIRouter, Depends

from app.repositories.order_repository import OrderRepository

router = APIRouter()


@router.get("/orders/{order_id}")
def get_order(order_id: int, repo: OrderRepository = Depends(OrderRepository)):
    return repo.find_by_id(order_id)
