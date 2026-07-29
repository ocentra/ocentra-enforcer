import time


def test_poll() -> None:
    time.sleep(1)
    assert poll() == "ready"
