"""services/order_service.py -- a service taking a raw Session param."""
from sqlalchemy.orm import Session


class OrderService:
    def __init__(self, session: Session):
        self.session = session

    def find_by_id(self, order_id: int):
        return self.session.query(Order).filter_by(id=order_id).first()
