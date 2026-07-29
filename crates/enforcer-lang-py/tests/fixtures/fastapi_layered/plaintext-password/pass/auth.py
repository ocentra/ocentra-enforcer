"""services/auth_service.py -- password hashed with bcrypt, never stored plaintext."""
import bcrypt


class AuthService:
    def register(self, username: str, password: str):
        hashed = bcrypt.hashpw(password.encode("utf-8"), bcrypt.gensalt())
        self.users[username] = {"password_hash": hashed}

    def login(self, username: str, password: str) -> bool:
        stored = self.users[username]["password_hash"]
        return bcrypt.checkpw(password.encode("utf-8"), stored)
