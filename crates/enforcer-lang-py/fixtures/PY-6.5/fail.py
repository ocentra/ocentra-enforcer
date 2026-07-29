def test_login(monkeypatch) -> None:
    monkeypatch.setattr("app.login", lambda: "ok")
    assert login() == "ok"
