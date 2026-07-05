//! e-pack-python — the FastAPI layered / clean-architecture rule family.
//!
//! Every rule here mechanizes one row of the workpack's Requirement
//! Checklist: layering/DI discipline between `routers/**`, `services/**`,
//! `workflows/**`, `domain/**` and the persistence layer, plus a small
//! Python-security slice (plaintext password storage, insecure random
//! tokens, wildcard CORS). StrEnum/enum-location and size/shape (nesting
//! depth, catch-all utils) are DELIBERATELY NOT re-implemented here — those
//! are consumed read-only from d16 (`enforcer_lang_common::rules::fsm`) and
//! d22 (`enforcer_lang_common::rules::size_shape`); the workpack's rows for
//! those two concerns are backed by `FSM-ENUMLOC.1` and the `SIZE-*`
//! records already registered against `enforcer-lang-common`.
//!
//! Like d16/d22 (and consistent with this workspace having zero
//! tree-sitter/AST dependency anywhere), every check here is a line/keyword
//! scan over source text, guarded by path-segment awareness (`routers/`,
//! `services/`, `workflows/`, `domain/`) so a rule only fires in the layer
//! it targets. "Symbol-level" is approximated by requiring an actual
//! import/call/annotation shape (e.g. `from ... import XRepository`,
//! `XRepository(`, `: Session`) rather than a bare substring match, so a
//! prose comment mentioning the same word in a pass fixture stays clean.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The fixed parts of one rule's finding: id, severity, title. Mirrors
/// `enforcer-lang-common`'s `rules::fsm` helper shape.
struct FindingSpec<'a> {
    rule_id: &'a RuleId,
    severity: Severity,
    title: &'a str,
}

fn finding(spec: &FindingSpec<'_>, detail: String, input: &ValidationInput<'_>, line: u32) -> Finding {
    Finding {
        rule_id: spec.rule_id.clone(),
        severity: spec.severity,
        title: spec.title.to_owned(),
        detail,
        file: input.file.clone(),
        line,
        snippet: None,
    }
}

/// True when `path` has a `routers/` (or `router/`) path segment.
fn in_layer(path: &str, segment: &str) -> bool {
    path.contains(&format!("/{segment}/")) || path.starts_with(&format!("{segment}/"))
}

/// 1-based line number of the first line whose trimmed text contains
/// `marker`, or `None` when absent.
fn first_line_with(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// True when any line's code (text before a `#` comment marker, if any)
/// contains `marker` — so a bare prose comment mentioning the word does
/// NOT count as a reference, only actual code does.
fn code_contains(source: &str, marker: &str) -> bool {
    source.lines().any(|line| code_part(line).contains(marker))
}

/// First 1-based line number whose CODE portion (not a trailing `#`
/// comment) contains `marker`.
fn first_code_line_with(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| code_part(line).contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// Strip a trailing `#`-comment from one line of Python source. Naive (no
/// string-literal awareness) but sufficient for this family's marker-based
/// checks, matching the precision level `enforcer-lang-common::rules::fsm`
/// already ships at.
fn code_part(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

/// PYFA-1.1 (`py-fastapi-no-repo-in-routers`) — a `routers/**` module
/// referencing a `*Repository` symbol (import or instantiation) is
/// flagged; a router depending only on a service stays clean.
pub struct NoRepoInRoutersValidator {
    rule_id: RuleId,
}

impl NoRepoInRoutersValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-1.1".parse()?,
        })
    }
}

impl Validator for NoRepoInRoutersValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "routers") {
            return Vec::new();
        }
        for (idx, line) in input.source.lines().enumerate() {
            let code = code_part(line);
            let references_repo = (code.contains("import") && code.contains("Repository"))
                || code.contains("Repository(");
            if references_repo {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: router references a Repository symbol",
                    },
                    format!(
                        "`{path}` references a `*Repository` symbol directly; routers must \
                         depend on a service, not the persistence layer. Fix: inject a service \
                         (e.g. `Depends(OrderService)`) and let the service own the repository."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-2.1 (`py-fastapi-no-session-in-services`) — a `Session`/
/// `AsyncSession` param or use inside `services/**` is flagged; a service
/// taking a repository stays clean.
pub struct NoSessionInServicesValidator {
    rule_id: RuleId,
}

impl NoSessionInServicesValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-2.1".parse()?,
        })
    }
}

impl Validator for NoSessionInServicesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "services") {
            return Vec::new();
        }
        for (idx, line) in input.source.lines().enumerate() {
            let code = code_part(line);
            let uses_session = code.contains(": Session")
                || code.contains(": AsyncSession")
                || code.contains("Session)")
                || code.contains("AsyncSession)");
            if uses_session {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: service takes a raw Session",
                    },
                    format!(
                        "`{path}` takes a `Session`/`AsyncSession` parameter directly; services \
                         must depend on a repository, not the ORM session. Fix: inject the \
                         repository instead and let it own the session."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-3.1 (`py-fastapi-no-transaction-in-services`) — `commit()`/
/// `begin()`/`session.rollback()` inside `services/**` is flagged; a
/// service that leaves tx control at the boundary/unit-of-work stays clean.
pub struct NoTransactionInServicesValidator {
    rule_id: RuleId,
}

impl NoTransactionInServicesValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-3.1".parse()?,
        })
    }
}

impl Validator for NoTransactionInServicesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "services") {
            return Vec::new();
        }
        for marker in [".commit(", ".begin(", ".rollback("] {
            if let Some(line) = first_code_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: service owns transaction control",
                    },
                    format!(
                        "`{path}` calls `{}` directly; transaction boundaries (commit/begin/\
                         rollback) must be owned by the request boundary or a unit-of-work, not \
                         a service. Fix: move the commit/rollback out to the boundary.",
                        marker.trim_end_matches('(')
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-4.1 (`py-fastapi-no-orm-models-in-services`) — importing an ORM
/// model class into `services/**` is flagged; a service using domain DTOs
/// stays clean.
pub struct NoOrmModelsInServicesValidator {
    rule_id: RuleId,
}

impl NoOrmModelsInServicesValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-4.1".parse()?,
        })
    }
}

impl Validator for NoOrmModelsInServicesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "services") {
            return Vec::new();
        }
        if let Some(line) = first_code_line_with(input.source, "from app.models") {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "fastapi-layered: service imports an ORM model",
                },
                format!(
                    "`{path}` imports directly from `app.models`; services must use domain \
                     DTOs, not ORM model classes. Fix: define/use a domain DTO instead."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// PYFA-5.1 (`py-fastapi-no-sqlalchemy-in-routers`) — `from sqlalchemy` /
/// `select(`/`.query(` inside `routers/**` is flagged; a router delegating
/// to a service stays clean.
pub struct NoSqlalchemyInRoutersValidator {
    rule_id: RuleId,
}

impl NoSqlalchemyInRoutersValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-5.1".parse()?,
        })
    }
}

impl Validator for NoSqlalchemyInRoutersValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "routers") {
            return Vec::new();
        }
        for marker in ["from sqlalchemy", "select(", ".query("] {
            if let Some(line) = first_code_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: router issues a SQLAlchemy query",
                    },
                    format!(
                        "`{path}` uses `{marker}` directly; routers must delegate persistence \
                         to a service, never issue SQLAlchemy queries themselves. Fix: move the \
                         query into a repository behind a service."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-6.1 (`py-fastapi-httpexception-location`) — `raise HTTPException`
/// outside `routers/**` (e.g. in services/domain) is flagged; raised only
/// in routers stays clean.
pub struct HttpExceptionLocationValidator {
    rule_id: RuleId,
}

impl HttpExceptionLocationValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-6.1".parse()?,
        })
    }
}

impl Validator for HttpExceptionLocationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if in_layer(path, "routers") {
            return Vec::new();
        }
        if let Some(line) = first_code_line_with(input.source, "raise HTTPException") {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "fastapi-layered: HTTPException raised outside routers/",
                },
                format!(
                    "`{path}` raises `HTTPException` outside `routers/**`; HTTP-shaped errors \
                     belong only at the router boundary. Fix: raise a domain error here and \
                     translate it to `HTTPException` in the router."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// PYFA-7.1 (`py-fastapi-no-repos-in-workflows`) — a `workflows/**` module
/// using a repository directly is flagged; a workflow calling a service
/// stays clean.
pub struct NoReposInWorkflowsValidator {
    rule_id: RuleId,
}

impl NoReposInWorkflowsValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-7.1".parse()?,
        })
    }
}

impl Validator for NoReposInWorkflowsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "workflows") {
            return Vec::new();
        }
        for (idx, line) in input.source.lines().enumerate() {
            let code = code_part(line);
            let references_repo = (code.contains("import") && code.contains("Repository"))
                || code.contains("Repository(");
            if references_repo {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: workflow references a Repository symbol",
                    },
                    format!(
                        "`{path}` references a `*Repository` symbol directly; workflows must \
                         call a service, not the persistence layer. Fix: inject a service."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-8.1 (`py-fastapi-models-mapped`) — a SQLAlchemy model column not
/// typed with `Mapped[...]` is flagged (bare `Column(` assignment);
/// `Mapped[int]`/`mapped_column` stays clean.
pub struct ModelsMappedValidator {
    rule_id: RuleId,
}

impl ModelsMappedValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-8.1".parse()?,
        })
    }
}

impl Validator for ModelsMappedValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        for (idx, line) in input.source.lines().enumerate() {
            let code = code_part(line);
            let trimmed = code.trim_start();
            let is_bare_column_assign = trimmed.contains(" = Column(") && !code.contains("Mapped[");
            if is_bare_column_assign {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: model column not typed with Mapped[...]",
                    },
                    format!(
                        "`{path}` declares a column with bare `Column(...)`, not `Mapped[T] = \
                         mapped_column(...)`. Fix: use SQLAlchemy 2.0 `Mapped[...]` typing."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-9.1 (`py-fastapi-domain-purity`) — `domain/**` importing FastAPI/
/// HTTP or raising `HTTPException` is flagged; a pure domain stays clean.
pub struct DomainPurityValidator {
    rule_id: RuleId,
}

impl DomainPurityValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-9.1".parse()?,
        })
    }
}

impl Validator for DomainPurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_layer(path, "domain") {
            return Vec::new();
        }
        for marker in ["from fastapi", "import fastapi", "raise HTTPException"] {
            if let Some(line) = first_code_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: domain layer depends on FastAPI/HTTP",
                    },
                    format!(
                        "`{path}` uses `{marker}`; `domain/**` must stay framework-pure — no \
                         FastAPI import and no HTTP-shaped exception. Fix: raise a plain domain \
                         error type instead, translated to HTTP at the router boundary."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-10.1 (`py-fastapi-no-sync-http`) — `requests.` / a sync
/// `httpx.Client` used in an async function is flagged; `httpx.AsyncClient`
/// with `await` stays clean.
pub struct NoSyncHttpValidator {
    rule_id: RuleId,
}

impl NoSyncHttpValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-10.1".parse()?,
        })
    }
}

impl Validator for NoSyncHttpValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let is_async_context = input.source.contains("async def");
        if !is_async_context {
            return Vec::new();
        }
        for marker in ["requests.get(", "requests.post(", "requests.put(", "httpx.Client("] {
            if let Some(line) = first_code_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: sync HTTP call in an async request path",
                    },
                    format!(
                        "`{path}` calls `{marker}` inside an `async def` request path; a \
                         blocking sync HTTP client stalls the event loop. Fix: use \
                         `httpx.AsyncClient` with `await`."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-11.1 (`py-fastapi-no-direct-repo-instantiation`) — a
/// `SomeRepository(...)` constructed inline (assignment, not a `Depends(...)`
/// default) is flagged; injected via `Depends`/DI stays clean.
pub struct NoDirectRepoInstantiationValidator {
    rule_id: RuleId,
}

impl NoDirectRepoInstantiationValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-11.1".parse()?,
        })
    }
}

impl Validator for NoDirectRepoInstantiationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        for (idx, line) in input.source.lines().enumerate() {
            let code = code_part(line);
            let trimmed = code.trim_start();
            let is_inline_construction = trimmed.contains("Repository(")
                && !trimmed.contains("Depends(")
                && !trimmed.contains("class ")
                && !trimmed.contains("def ");
            if is_inline_construction {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered: repository constructed inline",
                    },
                    format!(
                        "`{path}` constructs a `*Repository(...)` inline rather than receiving \
                         it via dependency injection. Fix: accept the repository as a \
                         constructor/`Depends`-injected parameter."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-12.1 (`py-fastapi-plaintext-password`) — storing or comparing a
/// plaintext password (a `password` field/compare with no `hash`/`bcrypt`/
/// `argon2` marker anywhere in the file) is flagged; bcrypt/argon2 usage
/// stays clean.
pub struct PlaintextPasswordValidator {
    rule_id: RuleId,
}

impl PlaintextPasswordValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-12.1".parse()?,
        })
    }
}

impl Validator for PlaintextPasswordValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let stores_password_field = code_contains(input.source, "\"password\":")
            || code_contains(input.source, "'password':")
            || code_contains(input.source, "[\"password\"]");
        if !stores_password_field {
            return Vec::new();
        }
        let uses_secure_hashing = input.source.contains("bcrypt")
            || input.source.contains("argon2")
            || input.source.contains("password_hash")
            || input.source.contains("scrypt")
            || input.source.contains("pbkdf2");
        if uses_secure_hashing {
            return Vec::new();
        }
        if let Some(line) = first_code_line_with(input.source, "password") {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "fastapi-layered/security: plaintext password store or compare",
                },
                format!(
                    "`{path}` stores or compares a password with no hashing (bcrypt/argon2) \
                     anywhere in the file. Fix: hash with `bcrypt`/`argon2` before storing, and \
                     compare via the library's constant-time check."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// PYFA-12.2 (`py-fastapi-insecure-random-token`) — `random.*` used to mint
/// a token is flagged; `secrets.token_hex`/`secrets.token_urlsafe` stays
/// clean.
pub struct InsecureRandomTokenValidator {
    rule_id: RuleId,
}

impl InsecureRandomTokenValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-12.2".parse()?,
        })
    }
}

impl Validator for InsecureRandomTokenValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let mentions_token = input.source.contains("token");
        if !mentions_token {
            return Vec::new();
        }
        for marker in ["random.choice(", "random.randint(", "random.random("] {
            if let Some(line) = first_code_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered/security: insecure random token generation",
                    },
                    format!(
                        "`{path}` uses `{marker}` to build a token; the `random` module is not \
                         cryptographically secure. Fix: use `secrets.token_hex`/\
                         `secrets.token_urlsafe` instead."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// PYFA-12.3 (`py-fastapi-cors-wildcard`) — `allow_origins=["*"]` is
/// flagged; an explicit origin list stays clean.
pub struct CorsWildcardValidator {
    rule_id: RuleId,
}

impl CorsWildcardValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "PYFA-12.3".parse()?,
        })
    }
}

impl Validator for CorsWildcardValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        for marker in ["allow_origins=[\"*\"]", "allow_origins=['*']"] {
            if let Some(line) = first_line_with(input.source, marker) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fastapi-layered/security: CORS wildcard origin",
                    },
                    format!(
                        "`{path}` sets `allow_origins=[\"*\"]`; a wildcard CORS origin combined \
                         with credentials is a cross-origin credential leak. Fix: list explicit \
                         allowed origins."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// Build every validator this module registers, in the order the workpack
/// lists them.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, enforcer_core::error::DecodeError> {
    Ok(vec![
        Box::new(NoRepoInRoutersValidator::new()?),
        Box::new(NoSessionInServicesValidator::new()?),
        Box::new(NoTransactionInServicesValidator::new()?),
        Box::new(NoOrmModelsInServicesValidator::new()?),
        Box::new(NoSqlalchemyInRoutersValidator::new()?),
        Box::new(HttpExceptionLocationValidator::new()?),
        Box::new(NoReposInWorkflowsValidator::new()?),
        Box::new(ModelsMappedValidator::new()?),
        Box::new(DomainPurityValidator::new()?),
        Box::new(NoSyncHttpValidator::new()?),
        Box::new(NoDirectRepoInstantiationValidator::new()?),
        Box::new(PlaintextPasswordValidator::new()?),
        Box::new(InsecureRandomTokenValidator::new()?),
        Box::new(CorsWildcardValidator::new()?),
    ])
}
