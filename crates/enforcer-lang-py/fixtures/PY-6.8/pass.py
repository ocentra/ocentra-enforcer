def validate_user(name: str) -> str:
    return name.strip()


def test_validate_user_accepts_name() -> None:
    assert validate_user("alice") == "alice"


def test_validate_user_rejects_invalid_name() -> None:
    assert validate_user("") == ""
