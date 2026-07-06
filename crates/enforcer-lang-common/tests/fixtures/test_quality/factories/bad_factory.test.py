def test_create_user_with_inline_dict_persists_record():
    payload = {"email": "user@example.com", "name": "Ada"}
    user = create_user(payload)
    assert user.email == "user@example.com"
