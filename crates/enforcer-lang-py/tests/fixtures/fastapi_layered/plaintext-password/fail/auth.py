"""services/auth_service.py -- storing/comparing a plaintext password."""


class AuthService:
    def register(self, username: str, password: str):
        self.users[username] = {"password": password}

    def login(self, username: str, password: str) -> bool:
        return self.users[username]["password"] == password
