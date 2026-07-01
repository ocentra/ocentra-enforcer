def parse(value: str) -> str:
    try:
        return value.strip()
    except Exception:
        return "PY-4.11"
