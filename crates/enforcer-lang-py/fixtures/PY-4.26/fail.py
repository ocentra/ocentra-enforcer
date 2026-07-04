import asyncio


def schedule() -> None:
    asyncio.create_task(work())
