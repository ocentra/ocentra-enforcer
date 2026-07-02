import {
  addViolation,
  hashCommentText,
  isPythonConfigBoundary,
  isPythonTestPath,
  isWeakPythonAssert,
  pythonAnyPattern,
  pythonBareExceptPattern,
  pythonBroadExceptPattern,
  pythonCoroutineCallPattern,
  pythonCreateTaskPattern,
  pythonDataclassPattern,
  pythonDynamicExecPattern,
  pythonDynamicImportPattern,
  pythonEnvReadPattern,
  pythonFunctionPattern,
  pythonMutableDefaultPattern,
  pythonNakedDomainAliasPattern,
  pythonNamedTuplePattern,
  pythonNaiveDatetimePattern,
  pythonOsSystemPattern,
  pythonParentRelativeImportPattern,
  pythonPickleLoadsPattern,
  pythonPrintPattern,
  pythonRawJsonDictPattern,
  pythonRequestsCallPattern,
  pythonRuntimeAssertPattern,
  pythonSleepPattern,
  pythonSubprocessShellPattern,
  pythonWildcardImportPattern,
  pythonYamlUnsafeLoadPattern,
} from "./generic-scanner-shared.mjs";

function addPythonViolation(context, lineNo, ruleId, detail, source) {
  addViolation(
    context.violations,
    context.root,
    context.filePath,
    lineNo,
    ruleId,
    detail,
    source,
  );
}

export function scanPythonSuppressions(context, line, lineNo) {
  const comment = hashCommentText(line);
  if (/#\s*noqa\b|\bpylint:\s*disable\b/iu.test(comment)) {
    addPythonViolation(context, lineNo, "PY-1.1", "Python lint suppression found.", line);
  }
  if (/#\s*type:\s*ignore\b/iu.test(comment)) {
    addPythonViolation(context, lineNo, "PY-1.2", "Python type-ignore suppression found.", line);
  }
}

export function scanPythonDomainModelRules(context, line, lineNo) {
  const nakedDomainAlias = line.match(pythonNakedDomainAliasPattern);
  if (nakedDomainAlias) {
    addPythonViolation(context, lineNo, "PY-1.3", `naked domain string alias ${nakedDomainAlias[1]}`, line);
    addPythonViolation(context, lineNo, "PY-4.5", `raw str ID alias ${nakedDomainAlias[1]}`, line);
  }

  if (pythonAnyPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.1", "Python Any usage found.", line);
    if (/\bdict\s*\[\s*str\s*,\s*(?:typing\.)?Any\s*\]/u.test(line)) {
      addPythonViolation(context, lineNo, "PY-4.4", "dict[str, Any] domain API found.", line);
    }
  }
  if (/\bTypedDict\b/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.7", "TypedDict domain model found.", line);
  }
  if (/\bBaseModel\b/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.8", "Pydantic BaseModel domain model found.", line);
  }
  if (/\b(?:Optional\s*\[|None\s*\|)/u.test(line) && /^\s*[A-Za-z_]\w*\s*:/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.9", "Optional field found in domain model.", line);
  }
  if (
    !context.isTestPath
    && /^\s*[A-Z_][A-Z0-9_]*\s*=\s*(?:\[\s*\]|\{\s*\}|set\(\s*\)|dict\(\s*\)|list\(\s*\))/u.test(line)
  ) {
    addPythonViolation(context, lineNo, "PY-4.21", "global mutable state found.", line);
  }
  const dataclassMatch = line.match(pythonDataclassPattern);
  if (dataclassMatch) {
    const args = dataclassMatch.groups?.args ?? "";
    if (!/\bfrozen\s*=\s*True\b/u.test(args) || !/\bslots\s*=\s*True\b/u.test(args)) {
      addPythonViolation(context, lineNo, "PY-4.32", "dataclass lacks frozen=True and slots=True.", line);
    }
  }
  if (pythonNamedTuplePattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.33", "NamedTuple or tuple domain record found.", line);
  }
}

export function scanPythonFunctionRules(context, line, lineNo) {
  const functionMatch = pythonFunctionPattern.exec(line);
  if (!functionMatch || context.isTestPath) return;
  const params = functionMatch[2]
    .split(",")
    .map((param) => param.trim())
    .filter(Boolean)
    .filter((param) => !/^(?:self|cls)(?:\s|$)/u.test(param));
  const untypedParam = params.find((param) => !param.includes(":"));
  if (untypedParam || !functionMatch[3]) {
    addPythonViolation(
      context,
      lineNo,
      "PY-4.2",
      "Python function is missing parameter or return type annotations.",
      line,
    );
  }
  if (!functionMatch[3]) {
    addPythonViolation(context, lineNo, "PY-4.3", "Python function is missing a return annotation.", line);
  }
  if (/(?:^|[,\s(])(?:\w+_)?(?:id|key|count|state|enabled|flag|status)\s*:\s*(?:str|int|bool)\b/iu.test(functionMatch[2])) {
    addPythonViolation(context, lineNo, "PY-4.6", "raw domain-like parameter type found.", line);
  }
  if (pythonRawJsonDictPattern.test(functionMatch[2])) {
    addPythonViolation(context, lineNo, "PY-4.34", "raw JSON dict domain input found.", line);
  }
  if (pythonMutableDefaultPattern.test(functionMatch[2])) {
    addPythonViolation(context, lineNo, "PY-4.10", "Mutable default argument found.", line);
  }
}

export function scanPythonSafetyRules(context, line, lineNo) {
  if (pythonBroadExceptPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.11", "Broad except Exception handler found.", line);
  }
  if (pythonBareExceptPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.12", "Bare except handler found.", line);
  }
  if (pythonBroadExceptPattern.test(line) || pythonBareExceptPattern.test(line)) {
    const nextSignificant = context.lines
      .slice(context.index + 1, context.index + 4)
      .find((candidate) => candidate.trim() !== "" && !candidate.trim().startsWith("#"));
    if (/^\s*pass\s*(?:#.*)?$/u.test(nextSignificant ?? "")) {
      addPythonViolation(context, lineNo + 1, "PY-4.13", "except block contains pass.", nextSignificant);
    }
  }
  if (pythonPrintPattern.test(line) && !context.isTestPath) {
    addPythonViolation(context, lineNo, "PY-4.14", "print debugging found in Python source.", line);
  }
  if (pythonRuntimeAssertPattern.test(line) && !context.isTestPath) {
    addPythonViolation(context, lineNo, "PY-4.15", "runtime assert found in Python source.", line);
  }
  if (pythonDynamicExecPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.16", "eval/exec/compile found.", line);
  }
  if (pythonSubprocessShellPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.17", "subprocess shell=True found.", line);
  }
  if (pythonOsSystemPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.18", "os.system found.", line);
  }
  if (pythonPickleLoadsPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.19", "pickle.loads found.", line);
  }
  if (pythonYamlUnsafeLoadPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.20", "yaml.load without SafeLoader found.", line);
  }
  if (pythonDynamicImportPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.22", "dynamic import found.", line);
  }
  if (pythonNaiveDatetimePattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.23", "naive datetime call found.", line);
  }
  if (pythonSleepPattern.test(line) && (context.isTestPath || /async\s+def/u.test(context.priorWindow))) {
    addPythonViolation(
      context,
      lineNo,
      context.isTestPath ? "PY-6.7" : "PY-4.24",
      context.isTestPath ? "sleep found in Python test." : "sleep found in async Python code.",
      line,
    );
  }
  if (pythonRequestsCallPattern.test(line) && !/\btimeout\s*=/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.25", "requests call without timeout found.", line);
  }
  if (pythonCreateTaskPattern.test(line) && !/^\s*(?:\w+\s*=|return\s+|await\s+)/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.26", "asyncio.create_task result is not tracked.", line);
  }
  if (pythonCoroutineCallPattern.test(line) && !context.isTestPath) {
    addPythonViolation(context, lineNo, "PY-4.27", "coroutine-like call is not awaited or returned.", line);
  }
  if (pythonParentRelativeImportPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.28", "parent-relative import found.", line);
  }
  if (pythonWildcardImportPattern.test(line)) {
    addPythonViolation(context, lineNo, "PY-4.29", "wildcard import found.", line);
    addPythonViolation(context, lineNo, "PY-4.30", "from-module wildcard import found.", line);
  }
  if (pythonEnvReadPattern.test(line) && !isPythonConfigBoundary(context.rel)) {
    addPythonViolation(context, lineNo, "PY-4.35", "environment read found outside config boundary.", line);
  }
}

export function scanPythonTestRules(context, line, lineNo) {
  if (!context.isTestPath) return;
  if (/@pytest\.mark\.(?:skip|skipif|xfail|focus)|pytest\.skip\s*\(|unittest\.skip/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-2.1", "Skipped or focused Python test found.", line);
    addPythonViolation(context, lineNo, "PY-6.1", "pytest skip/xfail marker found.", line);
  }
  if (/\b(?:monkeypatch|unittest\.mock|mock\.|Mock\s*\(|patch\s*\()/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-6.5", "monkeypatch/mock usage found in Python test.", line);
  }
  if (/\b(?:requests\.|httpx\.|urllib\.request|socket\.)/u.test(line)) {
    addPythonViolation(context, lineNo, "PY-6.6", "network API found in Python unit test.", line);
  }
  if (pythonRuntimeAssertPattern.test(line) && isWeakPythonAssert(line)) {
    addPythonViolation(context, lineNo, "PY-6.2", "weak Python assertion found.", line);
  }
}

