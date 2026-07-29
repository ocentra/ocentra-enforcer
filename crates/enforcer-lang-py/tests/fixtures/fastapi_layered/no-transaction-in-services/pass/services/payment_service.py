"""services/payment_service.py -- tx owned at the boundary/unit-of-work."""
from app.repositories.payment_repository import PaymentRepository


class PaymentService:
    def __init__(self, repo: PaymentRepository):
        self.repo = repo

    def charge(self, order_id: int, amount: int):
        # The unit-of-work boundary (outside this service) owns commit/rollback.
        self.repo.record_charge(order_id, amount)
