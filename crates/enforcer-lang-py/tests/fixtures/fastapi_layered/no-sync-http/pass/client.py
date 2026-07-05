"""services/payment_gateway_client.py -- async HTTP client, properly awaited."""
import httpx


async def charge_card(token: str, amount: int):
    async with httpx.AsyncClient() as client:
        response = await client.post(
            "https://gateway.example.com/charge",
            json={"token": token, "amount": amount},
        )
    return response.json()
