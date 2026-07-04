def test_fetch(fake_client: "FakeHttpClient") -> None:
    response = fake_client.get("https://example.com")
    assert response.status_code == 200
