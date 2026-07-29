"""routers/orders.py -- a router issuing SQLAlchemy queries directly."""
from fastapi import APIRouter
from sqlalchemy import select

router = APIRouter()


@router.get("/orders/{order_id}")
def get_order(order_id: int, db):
    stmt = select(Order).where(Order.id == order_id)
    return db.execute(stmt).scalar_one()
