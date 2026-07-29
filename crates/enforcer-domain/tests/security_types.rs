use enforcer_domain::security_types::{SecurityInvariantId, SecurityTestCategory};

#[test]
fn security_policy_tokens_accept_valid_values() -> Result<(), Box<dyn std::error::Error>> {
    let category = SecurityTestCategory::try_from("replay-idempotency".to_owned())?;
    let invariant = SecurityInvariantId::try_from("failure-not-reward".to_owned())?;

    assert_eq!(category.to_string(), "replay-idempotency");
    assert_eq!(invariant.to_string(), "failure-not-reward");
    Ok(())
}

#[test]
fn security_policy_tokens_reject_invalid_values() -> Result<(), Box<dyn std::error::Error>> {
    let category_error = match SecurityTestCategory::try_from("   ".to_owned()) {
        Err(error) => error,
        Ok(_) => {
            return Err(std::io::Error::other("blank security-test category was accepted").into())
        }
    };
    assert_eq!(category_error.path, "requiredTestCategory");
    assert_eq!(category_error.reason, "must not be empty");
    assert_eq!(category_error.input_hint, None);

    let invariant_error = match SecurityInvariantId::try_from("failure\nnot-reward".to_owned()) {
        Err(error) => error,
        Ok(_) => {
            return Err(std::io::Error::other(
                "control characters in a security invariant were accepted",
            )
            .into())
        }
    };
    assert_eq!(invariant_error.path, "securityInvariantId");
    assert_eq!(
        invariant_error.reason,
        "must not contain control characters"
    );
    assert_eq!(invariant_error.input_hint, None);
    Ok(())
}
