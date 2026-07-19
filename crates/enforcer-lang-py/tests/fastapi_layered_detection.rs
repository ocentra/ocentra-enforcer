//! e-pack-python acceptance proof (detection leg): every `py-fastapi-*`
//! validator in [`enforcer_lang_py::rules::fastapi_layered`] fires on its
//! fail fixture and stays silent on its pass fixture, via the standard
//! `enforcer-validator` fixture/parity harness (arc-05). Named proof row:
//! `python-fastapi-layered-detection`.

use std::path::PathBuf;

use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::harness::run_fixture_parity as assert_fixture_parity;
use enforcer_validator::validator::Validator;

/// Repo root: two levels up from this crate's manifest dir
/// (`crates/enforcer-lang-py` -> workspace root), matching the fixture
/// paths recorded in `crates/enforcer-rules/rules/fastapi-layered.json`.
fn repo_root() -> Result<RepoRoot, enforcer_domain::boundary::decode_error::DecodeError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    RepoRoot::try_from(path.as_path())
}

fn fixture_dir(name: &str) -> String {
    format!("crates/enforcer-lang-py/tests/fixtures/fastapi_layered/{name}")
}

/// One validator and its fail/pass fixture paths.
struct Case {
    validator: Box<dyn Validator>,
    fail: String,
    pass: String,
}

fn cases() -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    use enforcer_lang_py::rules::fastapi_layered::{
        CorsWildcardValidator, DomainPurityValidator, HttpExceptionLocationValidator,
        ModelsMappedValidator, NoDirectRepoInstantiationValidator, NoOrmModelsInServicesValidator,
        NoRepoInRoutersValidator, NoReposInWorkflowsValidator, NoSessionInServicesValidator,
        NoSqlalchemyInRoutersValidator, NoSyncHttpValidator, NoTransactionInServicesValidator,
        UnsafeAuthenticationStorageValidator, WeakRandomIdentifierGenerationValidator,
    };

    let dir = |name: &str, leaf: &str| format!("{}/{leaf}", fixture_dir(name));

    let cases: Vec<(Box<dyn Validator>, String, String)> = vec![
        (
            Box::new(NoRepoInRoutersValidator::new()?),
            dir("no-repo-in-routers", "fail/routers/orders.py"),
            dir("no-repo-in-routers", "pass/routers/orders.py"),
        ),
        (
            Box::new(NoSessionInServicesValidator::new()?),
            dir("no-session-in-services", "fail/services/order_service.py"),
            dir("no-session-in-services", "pass/services/order_service.py"),
        ),
        (
            Box::new(NoTransactionInServicesValidator::new()?),
            dir(
                "no-transaction-in-services",
                "fail/services/payment_service.py",
            ),
            dir(
                "no-transaction-in-services",
                "pass/services/payment_service.py",
            ),
        ),
        (
            Box::new(NoOrmModelsInServicesValidator::new()?),
            dir(
                "no-orm-models-in-services",
                "fail/services/order_service.py",
            ),
            dir(
                "no-orm-models-in-services",
                "pass/services/order_service.py",
            ),
        ),
        (
            Box::new(NoSqlalchemyInRoutersValidator::new()?),
            dir("no-sqlalchemy-in-routers", "fail/routers/orders.py"),
            dir("no-sqlalchemy-in-routers", "pass/routers/orders.py"),
        ),
        (
            Box::new(HttpExceptionLocationValidator::new()?),
            dir("httpexception-location", "fail/services/order_service.py"),
            dir("httpexception-location", "pass/routers/orders.py"),
        ),
        (
            Box::new(NoReposInWorkflowsValidator::new()?),
            dir(
                "no-repos-in-workflows",
                "fail/workflows/order_fulfillment.py",
            ),
            dir(
                "no-repos-in-workflows",
                "pass/workflows/order_fulfillment.py",
            ),
        ),
        (
            Box::new(ModelsMappedValidator::new()?),
            dir("models-mapped", "fail/models.py"),
            dir("models-mapped", "pass/models.py"),
        ),
        (
            Box::new(DomainPurityValidator::new()?),
            dir("domain-purity", "fail/domain/order.py"),
            dir("domain-purity", "pass/domain/order.py"),
        ),
        (
            Box::new(NoSyncHttpValidator::new()?),
            dir("no-sync-http", "fail/client.py"),
            dir("no-sync-http", "pass/client.py"),
        ),
        (
            Box::new(NoDirectRepoInstantiationValidator::new()?),
            dir("no-direct-repo-instantiation", "fail/service.py"),
            dir("no-direct-repo-instantiation", "pass/service.py"),
        ),
        (
            Box::new(UnsafeAuthenticationStorageValidator::new()?),
            dir("plaintext-password", "fail/auth.py"),
            dir("plaintext-password", "pass/auth.py"),
        ),
        (
            Box::new(WeakRandomIdentifierGenerationValidator::new()?),
            dir("insecure-random-token", "fail/tokens.py"),
            dir("insecure-random-token", "pass/tokens.py"),
        ),
        (
            Box::new(CorsWildcardValidator::new()?),
            dir("cors-wildcard", "fail/main.py"),
            dir("cors-wildcard", "pass/main.py"),
        ),
    ];
    Ok(cases
        .into_iter()
        .map(|(validator, fail, pass)| Case {
            validator,
            fail,
            pass,
        })
        .collect())
}

#[test]
fn every_fastapi_layered_rule_fires_on_fail_and_is_silent_on_pass(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    for Case {
        validator,
        fail,
        pass,
    } in cases()?
    {
        let fail: RelPath = fail.parse()?;
        let pass: RelPath = pass.parse()?;
        assert_fixture_parity(validator.as_ref(), &root, &fail, &pass).map_err(|source| {
            format!(
                "fixture parity failed for `{}`: {source}",
                validator.rule_id()
            )
        })?;
    }
    Ok(())
}

/// Hardening proof: each of the four rules whose detection was broadened
/// beyond a single literal marker fires on a realistic evasion the old,
/// narrower check missed, while its canonical pass fixture stays clean under
/// the broadened validator (no over-firing). Fail-variant fixtures live
/// under `<rule>/fail-variant/`; the clean leg reuses the rule's canonical
/// pass fixture.
#[test]
fn hardened_rules_catch_realistic_evasions() -> Result<(), Box<dyn std::error::Error>> {
    use enforcer_lang_py::rules::fastapi_layered::{
        CorsWildcardValidator, NoOrmModelsInServicesValidator, NoSyncHttpValidator,
        WeakRandomIdentifierGenerationValidator,
    };

    let dir = |name: &str, leaf: &str| format!("{}/{leaf}", fixture_dir(name));

    let raw_cases: Vec<(Box<dyn Validator>, String, String)> = vec![
        // PYFA-4.1: relative import from a non-`app` `models` package.
        (
            Box::new(NoOrmModelsInServicesValidator::new()?),
            dir(
                "no-orm-models-in-services",
                "fail-variant/services/order_service.py",
            ),
            dir(
                "no-orm-models-in-services",
                "pass/services/order_service.py",
            ),
        ),
        // PYFA-10.1: `requests.delete` (a verb the old list omitted).
        (
            Box::new(NoSyncHttpValidator::new()?),
            dir("no-sync-http", "fail-variant/client.py"),
            dir("no-sync-http", "pass/client.py"),
        ),
        // PYFA-12.2: `random.randrange` (a function the old list omitted).
        (
            Box::new(WeakRandomIdentifierGenerationValidator::new()?),
            dir("insecure-random-token", "fail-variant/tokens.py"),
            dir("insecure-random-token", "pass/tokens.py"),
        ),
        // PYFA-12.3: wildcard CORS written with surrounding whitespace.
        (
            Box::new(CorsWildcardValidator::new()?),
            dir("cors-wildcard", "fail-variant/main.py"),
            dir("cors-wildcard", "pass/main.py"),
        ),
    ];
    let cases = raw_cases
        .into_iter()
        .map(|(validator, fail, pass)| Case {
            validator,
            fail,
            pass,
        })
        .collect::<Vec<_>>();

    let root = repo_root()?;
    for Case {
        validator,
        fail,
        pass,
    } in cases
    {
        let fail: RelPath = fail.parse()?;
        let pass: RelPath = pass.parse()?;
        assert_fixture_parity(validator.as_ref(), &root, &fail, &pass).map_err(|source| {
            format!(
                "hardened-evasion parity failed for `{}`: {source}",
                validator.rule_id()
            )
        })?;
    }
    Ok(())
}

#[test]
fn family_covers_exactly_fourteen_rules() -> Result<(), Box<dyn std::error::Error>> {
    let validators = enforcer_lang_py::rules::fastapi_layered::validators()?;
    assert_eq!(
        validators.len(),
        14,
        "e-pack-python's fastapi_layered family must register exactly 14 validators"
    );
    Ok(())
}
