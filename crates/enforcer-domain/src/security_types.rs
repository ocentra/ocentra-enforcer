//! Canonical values shared by security-policy ingestion and consumers.

macro_rules! security_token {
    ($(#[$doc:meta])* $name:ident, $field:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[doc = "BRAND-INVARIANT: non-empty policy token without control characters."]
        pub struct $name(pub(crate) String);

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

security_token!(
    /// Required security-test category declared by an ingested policy.
    SecurityTestCategory,
    "requiredTestCategory"
);

security_token!(
    /// Economic or logic invariant declared by an ingested policy.
    SecurityInvariantId,
    "securityInvariantId"
);
