# companion-test: tests/test_foo.py
"""Order service: applies a discount to an order total."""


def apply_discount(order_total, discount_percent):
    """Apply a percentage discount to an order total."""
    return order_total - (order_total * discount_percent / 100)
