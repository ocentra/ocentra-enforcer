import yaml


def load(data: str) -> object:
    return yaml.safe_load(data)
