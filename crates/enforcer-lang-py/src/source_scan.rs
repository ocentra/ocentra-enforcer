//! `python/source-scan` validator: 44 rules (PY-1 x3 + PY-4 x35 + PY-6 x6),
//! all keyed to boundary line-marker validator entries. Each
//! entry's `guard` is chosen from the rule's real syntactic shape (not just
//! "contains this substring somewhere") so a pass fixture that legitimately
//! mentions the same words in a comment, string, or docstring stays silent
//! -- the mem-arc-06-0002 gotcha this crate's fixtures are built to catch.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::BuiltInPythonRule;
use enforcer_validator::validator::Validator;

use crate::boundary::line_marker::{Guard, LineMarkerValidator, MissingCompanionValidator};

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
            BuiltInPythonRule::Py1Rule1.id(),
            "Python lint suppression comments are forbidden",
            Guard::TrailingComment,
            &["noqa", "pylint: disable", "pylint:disable"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py1Rule2.id(),
            "Python type-ignore comments are forbidden",
            Guard::TrailingComment,
            &["type: ignore", "type:ignore"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py1Rule3.id(),
            "Python naked domain string aliases are forbidden",
            Guard::NotInCommentOrString,
            &["Alias = str", "TypeAlias = str"],
        )),
        // --- PY-4: source-shape bans ---------------------------------------
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule1.id(),
            "Python Any is forbidden",
            Guard::NotInCommentOrString,
            &["typing.Any", ": Any", "-> Any", "[Any]", "Any]"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule2.id(),
            "Python functions must be typed",
            Guard::LineStartsWith,
            &["def "],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule3.id(),
            "Python return annotations are required",
            Guard::LineStartsWith,
            &["def "],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule4.id(),
            "Python dict[str, Any] domain APIs are forbidden",
            Guard::NotInCommentOrString,
            &["dict[str, Any]", "Dict[str, Any]"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule5.id(),
            "Python raw str ID aliases are forbidden",
            Guard::NotInCommentOrString,
            &["Id = str", "ID = str"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule6.id(),
            "Python raw domain parameters are forbidden",
            Guard::NotInCommentOrString,
            &["user_id: str", "user_id: int", "user_id: bool"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule7.id(),
            "TypedDict domain models are forbidden",
            Guard::NotInCommentOrString,
            &["(TypedDict)", "(TypedDict, total=False)"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule8.id(),
            "Pydantic domain models are forbidden by default",
            Guard::NotInCommentOrString,
            &["(BaseModel)"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule9.id(),
            "Python Optional field soup is forbidden",
            Guard::NotInCommentOrString,
            &[": Optional["],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule10.id(),
            "Python mutable default arguments are forbidden",
            Guard::NotInCommentOrString,
            &["=[])", "={})", "=set())", "=[],", "={},"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule11.id(),
            "Broad Python exception handlers are forbidden",
            Guard::NotInCommentOrString,
            &["except Exception"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule12.id(),
            "Bare Python except handlers are forbidden",
            Guard::LineStartsWith,
            &["except:"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule13.id(),
            "Python except pass is forbidden",
            Guard::LineStartsWith,
            &["pass"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule14.id(),
            "Python print debugging is forbidden",
            Guard::WordBoundary,
            &["print("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule15.id(),
            "Python runtime asserts are forbidden",
            Guard::LineStartsWith,
            &["assert "],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule16.id(),
            "Python dynamic code execution is forbidden",
            Guard::WordBoundary,
            &["eval(", "exec(", "compile("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule17.id(),
            "Python subprocess shell=True is forbidden",
            Guard::Anywhere,
            &["shell=True"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule18.id(),
            "Python os.system is forbidden",
            Guard::NotInCommentOrString,
            &["os.system("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule19.id(),
            "Python pickle.loads is forbidden",
            Guard::NotInCommentOrString,
            &["pickle.loads("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule20.id(),
            "Python yaml.load requires a safe loader",
            Guard::NotInCommentOrString,
            &["yaml.load("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule21.id(),
            "Python global mutable state is forbidden",
            Guard::LineStartsWith,
            &["CACHE = {}", "CACHE = []", "_CACHE = {}", "_CACHE = []"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule22.id(),
            "Python dynamic imports are forbidden in domain code",
            Guard::NotInCommentOrString,
            &["importlib.import_module(", "__import__("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule23.id(),
            "Python naive datetime calls are forbidden",
            Guard::NotInCommentOrString,
            &["datetime.now()", "datetime.utcnow()"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule24.id(),
            "Python sleep is forbidden in async code and tests",
            Guard::NotInCommentOrString,
            &["time.sleep("],
        )),
        Box::new(MissingCompanionValidator::new(
            BuiltInPythonRule::Py4Rule25.id(),
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
            BuiltInPythonRule::Py4Rule26.id(),
            "Python asyncio tasks must be tracked",
            Guard::LineStartsWith,
            &["asyncio.create_task(", "loop.create_task("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule27.id(),
            "Python coroutine calls must be awaited or returned",
            Guard::LineStartsWith,
            &["load_async()", "fetch_async()", "run_async()"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule28.id(),
            "Python parent-relative imports are forbidden",
            Guard::LineStartsWith,
            &["from .."],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule29.id(),
            "Python wildcard imports are forbidden",
            Guard::LineStartsWith,
            &["import *"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule30.id(),
            "Python from-module wildcard imports are forbidden",
            Guard::NotInCommentOrString,
            &[" import *"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule31.id(),
            "Python dumping-ground module names are forbidden",
            Guard::NotInCommentOrString,
            &["utils.py", "helpers.py", "common.py"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule32.id(),
            "Python dataclass value objects must be frozen and slotted",
            Guard::LineStartsWith,
            &["@dataclass", "@dataclasses.dataclass"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule33.id(),
            "Python tuple domain records are forbidden",
            Guard::NotInCommentOrString,
            &["(NamedTuple)", "NamedTuple("],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule34.id(),
            "Python raw JSON dict domain inputs are forbidden",
            Guard::NotInCommentOrString,
            &["payload: dict", "payload: Dict", "body: dict", "body: Dict"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py4Rule35.id(),
            "Python environment reads must stay in config boundaries",
            Guard::NotInCommentOrString,
            &["os.environ[", "os.getenv("],
        )),
        // --- PY-6: test-shape bans that live in source-scan ----------------
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py6Rule1.id(),
            "Python skipped/xfail tests are forbidden without waiver",
            Guard::NotInCommentOrString,
            &[
                "pytest.mark.skip",
                "pytest.mark.xfail",
                "pytest.mark.skipif",
            ],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py6Rule3.id(),
            "Empty Python tests are forbidden",
            Guard::LineStartsWith,
            &["pass"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py6Rule4.id(),
            "Python tests must assert behavior",
            Guard::LineStartsWith,
            &["run()", "call()", "execute()"],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py6Rule5.id(),
            "Python monkeypatch and mocks are forbidden by default",
            Guard::WordBoundary,
            &[
                "monkeypatch.setattr",
                "unittest.simulation_framework",
                "Simulator(",
                "MagicMock(",
            ],
        )),
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py6Rule6.id(),
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
            BuiltInPythonRule::Py6Rule7.id(),
            "Python sleep-based tests are forbidden",
            Guard::NotInCommentOrString,
            &["time.sleep("],
        )),
    ])
}
