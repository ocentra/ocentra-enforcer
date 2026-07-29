class CacheService:
    def __init__(self) -> None:
        self._cache: dict[str, object] = {}

    def get(self, key: str) -> object:
        return self._cache.get(key)
