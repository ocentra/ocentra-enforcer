"""services/payment_service.py -- a service owning transaction control."""
from app.repositories.payment_repository import PaymentRepository


class PaymentService:
    def __init__(self, repo: PaymentRepository, session):
        self.repo = repo
        self.session = session

    def charge(self, order_id: int, amount: int):
        self.repo.record_charge(order_id, amount)
        self.session.commit()
