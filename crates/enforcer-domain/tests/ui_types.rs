use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ui_types::{UiAuthToken, UiBindHost};

#[test]
fn ui_bind_host_rejects_blank_and_control_input() {
    assert_eq!(
        UiBindHost::try_from(String::new()),
        Err(DecodeError::new("uiBindHost", "must not be empty"))
    );
    assert_eq!(
        UiBindHost::try_from("host name".to_owned()),
        Err(DecodeError::new(
            "uiBindHost",
            "must not contain whitespace or control characters"
        ))
    );
    assert_eq!(
        UiBindHost::try_from("127.0.0.1".to_owned()).map(|host| host.to_string()),
        Ok("127.0.0.1".to_owned())
    );
}

#[test]
fn ui_auth_token_rejects_empty_and_accepts_non_empty_secret() {
    assert_eq!(
        UiAuthToken::try_from(String::new()),
        Err(DecodeError::new("uiAuthToken", "must not be empty"))
    );
    assert_eq!(
        UiAuthToken::try_from("secret".to_owned()).map(|token| token.as_str().to_owned()),
        Ok("secret".to_owned())
    );
}
