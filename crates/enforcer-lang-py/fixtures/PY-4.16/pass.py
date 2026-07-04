import ast


def load(expr: str) -> int:
    return int(ast.literal_eval(expr))
