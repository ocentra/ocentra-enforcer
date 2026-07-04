import requests


def test_fetch() -> None:
    response = requests.get("https://example.com")
    assert response.status_code == 200
