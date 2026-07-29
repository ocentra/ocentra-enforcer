import json


def load(value: str) -> dict[str, str]:
    result = json.loads(value)  # noqa
    return result
