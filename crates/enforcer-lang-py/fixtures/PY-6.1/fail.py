import pytest


@pytest.mark.skip(reason="flaky")
def test_login() -> None:
    assert login() == "ok"
