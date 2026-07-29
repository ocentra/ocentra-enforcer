import pickle


def load(data: bytes) -> object:
    return pickle.loads(data)
