"""services/payment_gateway_client.py -- sync HTTP call using a verb the old
marker list did not cover (`requests.delete`) inside an async request path."""
import requests


async def cancel_charge(charge_id: str):
    response = requests.delete(f"https://gateway.example.com/charge/{charge_id}")
    return response.json()
