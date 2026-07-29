"""Companion test for services/foo.py::apply_discount."""

from services.foo import apply_discount


def test_apply_discount_happy_path_returns_discounted_total():
    assert apply_discount(100, 10) == 90


def test_apply_discount_zero_percent_returns_original_total():
    assert apply_discount(100, 0) == 100
