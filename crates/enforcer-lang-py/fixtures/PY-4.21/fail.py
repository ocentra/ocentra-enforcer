CACHE = {}


def load(key: str) -> object:
    return CACHE.get(key)
