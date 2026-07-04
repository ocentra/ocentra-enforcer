def load(value: str) -> str:
    if not value:
        raise ValueError("value is required")
    return value


def test_load_returns_value() -> None:
    assert load("alice") == "alice"
