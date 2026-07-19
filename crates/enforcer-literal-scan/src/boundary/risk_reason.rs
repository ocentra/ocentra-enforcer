use crate::{FileRole, RiskCategory};

const REASONS: [(&str, &str); 16] = [
    (
        "Secret-looking string literal found.",
        "Remove the secret from source and rotate exposed credentials.",
    ),
    (
        "Event or command-like literal found in code.",
        "Move it to an enum, schema constant, generated contract, or protocol owner.",
    ),
    (
        "Route or URL literal found in code.",
        "Use a route registry, URL newtype, config boundary, or protocol constants.",
    ),
    (
        "Protocol/header/media literal found.",
        "Prefer a protocol owner constant when this value is repeated or used outside boundary code.",
    ),
    (
        "ID/key-like literal found.",
        "Use branded keys, schema-owned field names, or generated contract constants.",
    ),
    (
        "State/status-like literal found.",
        "Use an enum, branded union, or state value object rather than magic strings.",
    ),
    (
        "Raw JSON blob string found in code.",
        "Move JSON into a typed fixture/schema boundary or construct typed values directly.",
    ),
    (
        "SQL-like string found in code.",
        "Keep SQL in query owners and parameterize inputs; review for injection risk.",
    ),
    (
        "Shell-like command string found in code.",
        "Use argv arrays and reviewed script/tooling boundaries instead of shell strings.",
    ),
    (
        "String literal appears in comparison/control flow.",
        "Replace with enum/constant/schema value if it encodes domain state.",
    ),
    (
        "Repeated string literal found across files.",
        "Move repeated domain/protocol values to one owner constant or generated contract.",
    ),
    (
        "Human-readable message literal found.",
        "Usually acceptable; no action unless repeated or protocol-like.",
    ),
    (
        "Test fixture string literal found.",
        "Usually acceptable; keep secrets and volatile snapshots out of tests.",
    ),
    (
        "Import/module specifier literal found.",
        "Ignored by literal-risk policy; import-boundary rules should handle architecture.",
    ),
    (
        "Literal appears in schema/config/protocol owner context.",
        "Usually acceptable if this file is the declared owner for the value.",
    ),
    (
        "Unclassified string literal found.",
        "Review only if repeated, used in domain state, or suspicious in context.",
    ),
];

pub(crate) fn reason_and_suggestion(
    category: RiskCategory,
    role: FileRole,
) -> (&'static str, &'static str) {
    if category == RiskCategory::HumanMessage && role == FileRole::Domain {
        return (
            reason_pair(category).0,
            "Usually acceptable for display/error text; verify it is not used as domain state.",
        );
    }
    reason_pair(category)
}

fn reason_pair(category: RiskCategory) -> (&'static str, &'static str) {
    let index = match category {
        RiskCategory::SecretLike => 0,
        RiskCategory::EventOrCommandName => 1,
        RiskCategory::RouteOrUrl => 2,
        RiskCategory::ProtocolHeaderOrMedia => 3,
        RiskCategory::IdOrKeyName => 4,
        RiskCategory::StateOrStatus => 5,
        RiskCategory::RawJsonBlob => 6,
        RiskCategory::SqlFragment => 7,
        RiskCategory::ShellFragment => 8,
        RiskCategory::MagicStringComparison => 9,
        RiskCategory::RepeatedLiteral => 10,
        RiskCategory::HumanMessage => 11,
        RiskCategory::TestFixture => 12,
        RiskCategory::ImportSpecifier => 13,
        RiskCategory::SchemaOwnerLiteral => 14,
        RiskCategory::UnknownLiteral => 15,
    };
    match REASONS.get(index) {
        Some(pair) => *pair,
        None => (
            "Unclassified string literal found.",
            "Review only if repeated, used in domain state, or suspicious in context.",
        ),
    }
}
