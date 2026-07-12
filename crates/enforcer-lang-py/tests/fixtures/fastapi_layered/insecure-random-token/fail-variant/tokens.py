"""services/token_service.py -- insecure token minting using a `random.*`
function the old marker list did not cover (`random.randrange`)."""
import random
import string


def generate_reset_token() -> str:
    alphabet = string.ascii_letters + string.digits
    return "".join(alphabet[random.randrange(len(alphabet))] for _ in range(32))
