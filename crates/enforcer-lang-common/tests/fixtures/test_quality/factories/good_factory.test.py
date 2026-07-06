def test_create_user_with_factory_persists_record():
    payload = make_user_payload(email="user@example.com", name="Ada")
    user = create_user(payload)
    assert user.email == "user@example.com"
