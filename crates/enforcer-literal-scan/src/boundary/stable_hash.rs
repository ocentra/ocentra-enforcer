use enforcer_domain::scan_types::LiteralStableHash;

pub(crate) fn stable_hash_key(text: &str) -> LiteralStableHash {
    LiteralStableHash::of_source(
        enforcer_domain::boundary::validation::ValidationSource::from_text(text),
    )
}
