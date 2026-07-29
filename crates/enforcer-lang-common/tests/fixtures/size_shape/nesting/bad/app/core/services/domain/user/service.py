class UserService:
    def get_user(self, user_id: str) -> dict:
        return {"id": user_id}
