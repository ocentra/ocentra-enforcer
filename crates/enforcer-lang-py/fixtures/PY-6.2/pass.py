def test_login() -> None:
    user = login()
    assert user.name == "alice"
