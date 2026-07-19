//! Raw security-policy token conversion boundary.

use crate::boundary::decode_error::DecodeError;
use crate::security_types::{SecurityInvariantId, SecurityTestCategory};

fn validate_token(field: &str, value: &str) -> Result<(), DecodeError> {
    if value.trim().is_empty() {
        return Err(DecodeError::new(field, "must not be empty"));
    }
    if value.chars().any(char::is_control) {
        return Err(DecodeError::new(
            field,
            "must not contain control characters",
        ));
    }
    Ok(())
}

impl TryFrom<String> for SecurityTestCategory {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_token("requiredTestCategory", &value)?;
        Ok(Self(value))
    }
}

impl From<SecurityTestCategory> for String {
    fn from(value: SecurityTestCategory) -> Self {
        value.0
    }
}

impl TryFrom<String> for SecurityInvariantId {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_token("securityInvariantId", &value)?;
        Ok(Self(value))
    }
}

impl From<SecurityInvariantId> for String {
    fn from(value: SecurityInvariantId) -> Self {
        value.0
    }
}
