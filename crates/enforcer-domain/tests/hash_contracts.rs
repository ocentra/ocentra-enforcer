use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::hashes::Sha256;

#[test]
fn accepts_domain_minted_hash_output() -> Result<(), DecodeError> {
    let minted = enforcer_domain::boundary::hash::validate(b"payload");
    let branded: Sha256 = minted.as_str().parse()?;
    assert_eq!(branded.as_str(), minted.as_str());
    assert_eq!(branded.hex().len(), 64);
    Ok(())
}

#[test]
fn rejects_malformed_digests() {
    let bad_cases = [
        "",
        "sha256:",
        "sha256:short",
        "deadbeef",
        // uppercase hex
        &format!("sha256:{}", "A".repeat(64)),
        // non-hex chars
        &format!("sha256:{}", "g".repeat(64)),
        // wrong prefix
        &format!("md5:{}", "a".repeat(64)),
    ];
    for bad in bad_cases {
        let outcome: Result<Sha256, _> = bad.parse();
        assert_eq!(
            outcome.as_ref().err().map(|error| error.path.as_str()),
            Some("sha256"),
            "should reject {bad:?}"
        );
    }
}

#[test]
fn record_wire_mapping_rejects_invalid_digest() -> Result<(), serde_json::Error> {
    let wire = format!("\"sha256:{}\"", "ab".repeat(32));
    let parsed: Sha256 = enforcer_domain::boundary::json::from_str(&wire)?;
    assert_eq!(enforcer_domain::boundary::json::to_string(&parsed)?, wire);
    let rejection = enforcer_domain::boundary::json::from_str::<Sha256>("\"sha256:short\"")
        .err()
        .ok_or_else(|| serde_json::Error::io(std::io::Error::other("short digest accepted")))?;
    assert_eq!(rejection.classify(), serde_json::error::Category::Data);
    Ok(())
}

/// Named oracle for `proof/schema/a05-sha256.txt`: `Sha256::of` hashes
/// content and mints a value that is itself accepted by `Sha256::parse`
/// (`of()`+`parse()` round-trip), and length/case/charset violations are
/// rejected via `FromStr`/`TryFrom` before a transport boundary encodes
/// the value.
#[test]
fn sha256_brand_decode() -> Result<(), DecodeError> {
    // `of()` mints a digest that round-trips through `parse()`.
    let minted = enforcer_domain::boundary::hash::validate(b"hello world");
    let reparsed: Sha256 = minted.as_str().parse()?;
    assert_eq!(minted, reparsed);
    assert_eq!(minted.hex().len(), 64);
    assert!(minted.hex().chars().all(|c| c.is_ascii_hexdigit()));
    assert!(minted.hex().chars().all(|c| !c.is_ascii_uppercase()));

    // Deterministic: same bytes -> same digest; different bytes -> different digest.
    assert_eq!(
        enforcer_domain::boundary::hash::validate(b"hello world"),
        enforcer_domain::boundary::hash::validate(b"hello world")
    );
    assert_ne!(
        enforcer_domain::boundary::hash::validate(b"hello world"),
        enforcer_domain::boundary::hash::validate(b"hello world!")
    );

    // Empty input is a valid (well-known) SHA-256 preimage, not a special case.
    let empty = enforcer_domain::boundary::hash::validate(b"");
    assert_eq!(
        empty.hex(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // Wrong length / case / charset all fail closed via FromStr/TryFrom.
    for bad in [
        "sha256:short",
        &format!("sha256:{}", "A".repeat(64)), // uppercase
        &format!("sha256:{}", "g".repeat(64)), // non-hex
        &format!("sha256:{}", "a".repeat(63)), // too short
        &format!("sha256:{}", "a".repeat(65)), // too long
    ] {
        let outcome: Result<Sha256, DecodeError> = bad.parse();
        assert_eq!(
            outcome.as_ref().err().map(|error| error.path.as_str()),
            Some("sha256"),
            "should reject {bad:?}"
        );
    }

    Ok(())
}

// Compile-reject fixture (acceptance criterion: "the private field makes
// a bare `String` populating a `Sha256` field a compile error"). This is
// proved at review/proof time, not by a runtime test -- Sha256's only
// field is private (`Sha256(String)` with no public tuple-index/ctor), so
// the snippet below does not compile if uncommented:
//
//     struct Holder {
//         digest: Sha256,
//     }
//     fn bad(raw: String) -> Holder {
//         Holder { digest: raw } // expected `Sha256`, found `String`
//     }
//
// fails with E0308 (mismatched types): there is no `From<String> for
// Sha256`/`Into<Sha256> for String` impl (only the reverse, fallible
// `TryFrom<String>`), so a bare `String` never satisfies a `Sha256`-typed
// field or parameter.
