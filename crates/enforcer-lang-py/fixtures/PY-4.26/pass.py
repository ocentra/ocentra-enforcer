import asyncio


class TaskSupervisor:
    def __init__(self) -> None:
        self._tasks: set[asyncio.Task[None]] = set()

    def schedule(self) -> None:
        task = asyncio.create_task(work())
        self._tasks.add(task)
