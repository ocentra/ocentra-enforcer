import importlib


def load(name: str) -> object:
    return importlib.import_module(name)
