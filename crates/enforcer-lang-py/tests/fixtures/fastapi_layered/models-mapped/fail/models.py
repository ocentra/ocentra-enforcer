"""models/order.py -- a SQLAlchemy model column not typed with Mapped[...]."""
from sqlalchemy import Column, Integer
from sqlalchemy.orm import DeclarativeBase


class Base(DeclarativeBase):
    pass


class Order(Base):
    __tablename__ = "orders"

    id = Column(Integer, primary_key=True)
    total = Column(Integer)
