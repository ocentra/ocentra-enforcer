import json


def load(value: str) -> dict[str, str]:
    # Parse the raw payload into a plain string mapping.
    result = json.loads(value)
    return result
