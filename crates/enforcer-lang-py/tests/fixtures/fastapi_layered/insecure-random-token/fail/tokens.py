"""services/token_service.py -- using random.* to mint a security token."""
import random
import string


def generate_reset_token() -> str:
    alphabet = string.ascii_letters + string.digits
    return "".join(random.choice(alphabet) for _ in range(32))
