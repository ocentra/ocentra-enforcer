//! `python/source-scan` validator: 44 rules (PY-1 x3 + PY-4 x35 + PY-6 x6),
//! all keyed to [`crate::line_marker::LineMarkerValidator`] entries. Each
//! entry's `guard` is chosen from the rule's real syntactic shape (not just
//! "contains this substring somewhere") so a pass fixture that legitimately
//! mentions the same words in a comment, string, or docstring stays silent
//! -- the mem-arc-06-0002 gotcha this crate's fixtures are built to catch.

use enforcer_core::error::DecodeError;
use enforcer_validator::validator::Validator;

use crate::line_marker::{Guard, LineMarkerValidator, MissingCompanionValidator};

/// Build every `python/source-scan`-keyed validator this crate registers.
/// Order matches the PY-1 / PY-4 / PY-6 rule inventory in the workpack.
///
/// Returns `Err` if any of this module's fixed `RuleId` literals fails to
/// parse -- fail-closed construction rather than an `unwrap`/`expect` on a
/// value this module asserts is always well-formed; the `registry_coverage`
/// test in `lib.rs` calls this and would surface such a typo immediately.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        // --- PY-1: suppression / alias bans -------------------------------
        Box::new(LineMarkerValidator::new(
            id("PY-1.1")?,
            "Python lint suppression comments are forbidden",
            Guard::TrailingComment,
            &["noqa", "pylint: disable", "pylint:disable"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-1.2")?,
            "Python type-ignore comments are forbidden",
            Guard::TrailingComment,
            &["type: ignore", "type:ignore"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-1.3")?,
            "Python naked domain string aliases are forbidden",
            Guard::NotInCommentOrString,
            &["Alias = str", "TypeAlias = str"],
        )),
        // --- PY-4: source-shape bans ---------------------------------------
        Box::new(LineMarkerValidator::new(
            id("PY-4.1")?,
            "Python Any is forbidden",
            Guard::NotInCommentOrString,
            &["typing.Any", ": Any", "-> Any", "[Any]", "Any]"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.2")?,
            "Python functions must be typed",
            Guard::LineStartsWith,
            &["def "],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.3")?,
            "Python return annotations are required",
            Guard::LineStartsWith,
            &["def "],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.4")?,
            "Python dict[str, Any] domain APIs are forbidden",
            Guard::NotInCommentOrString,
            &["dict[str, Any]", "Dict[str, Any]"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.5")?,
            "Python raw str ID aliases are forbidden",
            Guard::NotInCommentOrString,
            &["Id = str", "ID = str"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.6")?,
            "Python raw domain parameters are forbidden",
            Guard::NotInCommentOrString,
            &["user_id: str", "user_id: int", "user_id: bool"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.7")?,
            "TypedDict domain models are forbidden",
            Guard::NotInCommentOrString,
            &["(TypedDict)", "(TypedDict, total=False)"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.8")?,
            "Pydantic domain models are forbidden by default",
            Guard::NotInCommentOrString,
            &["(BaseModel)"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.9")?,
            "Python Optional field soup is forbidden",
            Guard::NotInCommentOrString,
            &[": Optional["],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.10")?,
            "Python mutable default arguments are forbidden",
            Guard::NotInCommentOrString,
            &["=[])", "={})", "=set())", "=[],", "={},"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.11")?,
            "Broad Python exception handlers are forbidden",
            Guard::NotInCommentOrString,
            &["except Exception"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.12")?,
            "Bare Python except handlers are forbidden",
            Guard::LineStartsWith,
            &["except:"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.13")?,
            "Python except pass is forbidden",
            Guard::LineStartsWith,
            &["pass"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.14")?,
            "Python print debugging is forbidden",
            Guard::WordBoundary,
            &["print("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.15")?,
            "Python runtime asserts are forbidden",
            Guard::LineStartsWith,
            &["assert "],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.16")?,
            "Python dynamic code execution is forbidden",
            Guard::WordBoundary,
            &["eval(", "exec(", "compile("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.17")?,
            "Python subprocess shell=True is forbidden",
            Guard::NotInCommentOrString,
            &["shell=True"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.18")?,
            "Python os.system is forbidden",
            Guard::NotInCommentOrString,
            &["os.system("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.19")?,
            "Python pickle.loads is forbidden",
            Guard::NotInCommentOrString,
            &["pickle.loads("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.20")?,
            "Python yaml.load requires a safe loader",
            Guard::NotInCommentOrString,
            &["yaml.load("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.21")?,
            "Python global mutable state is forbidden",
            Guard::LineStartsWith,
            &["CACHE = {}", "CACHE = []", "_CACHE = {}", "_CACHE = []"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.22")?,
            "Python dynamic imports are forbidden in domain code",
            Guard::NotInCommentOrString,
            &["importlib.import_module(", "__import__("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.23")?,
            "Python naive datetime calls are forbidden",
            Guard::NotInCommentOrString,
            &["datetime.now()", "datetime.utcnow()"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.24")?,
            "Python sleep is forbidden in async code and tests",
            Guard::NotInCommentOrString,
            &["time.sleep("],
        )),
        Box::new(MissingCompanionValidator::new(
            id("PY-4.25")?,
            "Python HTTP calls must set timeouts",
            &[
                "requests.get(",
                "requests.post(",
                "requests.put(",
                "requests.delete(",
            ],
            &["timeout="],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.26")?,
            "Python asyncio tasks must be tracked",
            Guard::LineStartsWith,
            &["asyncio.create_task(", "loop.create_task("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.27")?,
            "Python coroutine calls must be awaited or returned",
            Guard::LineStartsWith,
            &["load_async()", "fetch_async()", "run_async()"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.28")?,
            "Python parent-relative imports are forbidden",
            Guard::LineStartsWith,
            &["from .."],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.29")?,
            "Python wildcard imports are forbidden",
            Guard::LineStartsWith,
            &["import *"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.30")?,
            "Python from-module wildcard imports are forbidden",
            Guard::NotInCommentOrString,
            &[" import *"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.31")?,
            "Python dumping-ground module names are forbidden",
            Guard::NotInCommentOrString,
            &["utils.py", "helpers.py", "common.py"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.32")?,
            "Python dataclass value objects must be frozen and slotted",
            Guard::LineStartsWith,
            &["@dataclass", "@dataclasses.dataclass"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.33")?,
            "Python tuple domain records are forbidden",
            Guard::NotInCommentOrString,
            &["(NamedTuple)", "NamedTuple("],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.34")?,
            "Python raw JSON dict domain inputs are forbidden",
            Guard::NotInCommentOrString,
            &["payload: dict", "payload: Dict", "body: dict", "body: Dict"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-4.35")?,
            "Python environment reads must stay in config boundaries",
            Guard::NotInCommentOrString,
            &["os.environ[", "os.getenv("],
        )),
        // --- PY-6: test-shape bans that live in source-scan ----------------
        Box::new(LineMarkerValidator::new(
            id("PY-6.1")?,
            "Python skipped/xfail tests are forbidden without waiver",
            Guard::NotInCommentOrString,
            &[
                "pytest.mark.skip",
                "pytest.mark.xfail",
                "pytest.mark.skipif",
            ],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-6.3")?,
            "Empty Python tests are forbidden",
            Guard::LineStartsWith,
            &["pass"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-6.4")?,
            "Python tests must assert behavior",
            Guard::LineStartsWith,
            &["run()", "call()", "execute()"],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-6.5")?,
            "Python monkeypatch and mocks are forbidden by default",
            Guard::WordBoundary,
            &[
                "monkeypatch.setattr",
                "unittest.mock",
                "Mock(",
                "MagicMock(",
            ],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-6.6")?,
            "Python unit tests must not use the network",
            Guard::NotInCommentOrString,
            &[
                "requests.get(",
                "requests.post(",
                "httpx.get(",
                "httpx.post(",
            ],
        )),
        Box::new(LineMarkerValidator::new(
            id("PY-6.7")?,
            "Python sleep-based tests are forbidden",
            Guard::NotInCommentOrString,
            &["time.sleep("],
        )),
    ])
}

/// Parse one of this module's fixed `PY-*` literals into a [`RuleId`],
/// propagating (rather than unwrapping/panicking on) the vanishingly
/// unlikely case of a typo -- `all()`'s `Result` return makes that a
/// regular, testable failure instead of a panic.
fn id(raw: &str) -> Result<enforcer_domain::ids::RuleId, DecodeError> {
    raw.parse()
}
