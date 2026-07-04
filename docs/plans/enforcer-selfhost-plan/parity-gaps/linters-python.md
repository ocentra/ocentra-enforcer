# Parity gaps: linters-python

Delta of ADBP mechanical rules with NO or PARTIAL backing in `rules/rules.json`.
Fully-backed rules omitted: `subprocess shell=True` (backed: "Python subprocess shell=True is forbidden"), `os.system` (backed: same family snippet).

| ADBP point | ADBP source | Backed? (family or NO/PARTIAL) | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| `.replace(tzinfo=None)` strips tz — forbidden for audit timestamps | fastapi-layered/check_datetime_patterns.py | PARTIAL (naive-datetime rule covers `now()`/`utcnow()` only, not tzinfo-strip) | T1 | py.datetime.no-tzinfo-strip | `dt.replace(tzinfo=None)` in src | `dt.astimezone(timezone.utc)` |
| Enum discipline: no raw string comparisons against known enum values (`status == "active"`) | fastapi-layered/check_enum_discipline.py | NO | T2 | py.enum.no-raw-string-compare | `if status == "active":` | `if status == Status.ACTIVE:` |
| Every class in `enums/` must inherit from `StrEnum` | fastapi-layered/check_strenum_only.py | NO | T1 | py.enum.strenum-only | `class Status(Enum):` in enums/ | `class Status(StrEnum):` |
| Models must use SQLAlchemy 2.0 typed `Mapped[T]` (no bare `Mapped`, no legacy `Column`) | fastapi-layered/check_models_use_mapped.py | NO | T1 | py.orm.models-use-mapped | `name = Column(String)` / bare `Mapped` | `name: Mapped[str] = mapped_column()` |
| HTTPException must stay in main.py / exception handlers | fastapi-layered/check_no_httpexception_outside_handlers.py | NO | T1 | py.layer.httpexception-handlers-only | `raise HTTPException(...)` in services/ | domain exception raised in service |
| Services must not import the models/ORM layer | fastapi-layered/check_no_models_in_services.py | PARTIAL (generic multi-layer-import rule, not directional service->models) | T1 | py.layer.no-models-in-services | service imports `app.models.user` | service imports repository |
| Routers must depend on `*ServiceDep`, not `*RepoDep` | fastapi-layered/check_no_repodep_in_routers.py | NO | T1 | py.layer.no-repodep-in-routers | router uses `UserRepoDep` | router uses `UserServiceDep` |
| Workflows must not import repositories/ or models/ | fastapi-layered/check_no_repos_in_workflows.py | NO | T1 | py.layer.no-repos-in-workflows | workflow imports `repositories.user` | workflow imports service |
| Services must not accept `Session`/`AsyncSession`/`AsyncSessionFactory` | fastapi-layered/check_no_session_in_services.py | NO | T1 | py.layer.no-session-in-services | `def __init__(self, s: AsyncSession)` | `def __init__(self, repo: UserRepo)` |
| Routers must not import `sqlalchemy` | fastapi-layered/check_no_sqlalchemy_in_routers.py | NO | T1 | py.layer.no-sqlalchemy-in-routers | router `import sqlalchemy` | router imports service only |
| Services must not call `.commit()`/`.flush()`/`.rollback()` | fastapi-layered/check_no_transaction_in_services.py | NO | T1 | py.layer.no-transaction-in-services | `session.commit()` in service | commit in unit-of-work/repo |
| Every source file in a watched layer must have a test companion | fastapi-layered/check_test_companion_exists.py | NO | T1 | py.tests.companion-required | `services/user.py` with no `test_user.py` | source + matching test file |
| File/function/class size limits (with grandfather ratchet) | fastapi-layered/check_file_length.py | NO | T2 | py.size.file-function-class-limits | file exceeds line cap (not baselined) | file under cap |
| Every line <= 120 chars (incl. trailing-comment overflow) | fastapi-layered/check_line_length_strict.py | NO | T1 | py.style.line-length-120 | 130-char line | all lines <=120 |
| domain/agents/verification/audit must not import UI exceptions (streamlit, fastapi HTTPException, starlette) | clean-arch/check_domain_no_ui_exceptions.py | NO | T1 | py.cleanarch.no-ui-exceptions-in-domain | `from fastapi import HTTPException` in domain/ | domain-specific exception |
| domain/ must be pure — only stdlib/pydantic/networkx/pandas/numpy; forbid PydanticAI, statemachine, loguru, SQLAlchemy, Streamlit | clean-arch/check_domain_purity.py | NO | T1 | py.cleanarch.domain-purity | `import sqlalchemy` in domain/ | domain imports pydantic only |
| No sync HTTP libs in src/ (requests, urllib, urllib3, http.client) — use httpx async | clean-arch/check_no_sync_http.py | PARTIAL (a "requests" hit exists but not the src/-wide sync-HTTP ban incl. urllib/http.client) | T1 | py.http.no-sync-http | `import requests` in src | `import httpx` |
| No direct repository instantiation in domain/agents/engine/verification/audit — inject via constructor DI | clean-arch/check_repo_direct_instantiation.py | NO | T1 | py.cleanarch.no-direct-repo-instantiation | `UserRepo()` in engine/ | repo passed to `__init__` |
| SQL injection: no f-string/`.format()` in execute/raw calls (CWE-89) | security-python/check_sql_injection.py | NO | T1 | py.sec.sql-injection-fstring | `cursor.execute(f"... {table}")` | parameterized `execute(sql, params)` |
| Hardcoded secrets: sk-/ghp_/aws_/`password=` literal formats (CWE-798) | security-python/check_hardcoded_secrets.py | PARTIAL (generic inline/high-entropy secret rules exist; token-format prefixes sk-/ghp_/aws_ not covered) | T1 | py.sec.hardcoded-secret-formats | `key = "sk-abc123..."` | `key = os.environ["KEY"]` |
| Supply chain: suspicious setup.py imports (subprocess/os.system/requests), exfil `exec()`/`eval()`/`shell=True`, typosquat pkg names | security-python/check_supply_chain.py | NO | T2 | py.sec.supply-chain-setup-py | `setup.py` with `os.system(...)`/typosquat dep | clean setup.py / pyproject |
