"""services/token_service.py -- using secrets.token_hex for a security token."""
import secrets


def generate_reset_token() -> str:
    return secrets.token_hex(32)
