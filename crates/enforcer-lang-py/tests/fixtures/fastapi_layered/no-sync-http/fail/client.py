"""services/payment_gateway_client.py -- sync HTTP call in an async request path."""
import requests


async def charge_card(token: str, amount: int):
    response = requests.post("https://gateway.example.com/charge", json={"token": token, "amount": amount})
    return response.json()
