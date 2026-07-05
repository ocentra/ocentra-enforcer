"""routers/orders.py -- HTTPException raised only at the router boundary."""
from fastapi import APIRouter, Depends, HTTPException

from app.domain.errors import OrderNotFoundError
from app.services.order_service import OrderService

router = APIRouter()


@router.get("/orders/{order_id}")
def get_order(order_id: int, service: OrderService = Depends(OrderService)):
    try:
        return service.find_by_id(order_id)
    except OrderNotFoundError as exc:
        raise HTTPException(status_code=404, detail="order not found") from exc
