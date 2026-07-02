import path from "node:path";
import {
  addViolation,
  BAD_SOURCE_BASENAME_RE,
  COMMON_SECRET_RULES,
  ENV_PLACEHOLDER_ALLOWED,
  GENERATED_DOMAIN_PATH_RE,
  GENERATED_HASH_RE,
  GENERATED_OUTPUT_DIR_RE,
  GENERATED_PATH_RE,
  GENERATED_PROVENANCE_RE,
  GENERATED_SNAPSHOT_VOLATILE_RE,
  GENERATED_SUPPRESSION_RE,
  MOBILE_SECRET_CONFIG_RE,
  OPENAI_KEY_RE,
  SECRET_RE,
  SSH_KEY_PATH_RE,
  isCommandLikeLine,
  isEnforcerToolingPath,
  isGeneratedLikePath,
  isGeneratedMarkerSourceFile,
  isProductionSourcePath,
  isPythonTestPath,
  isTestPath,
  placeholderCommentPatterns,
  placeholderDirectPatterns,
  redact,
  redactOpenAiKey,
  sourceCommentText,
  weakAssertionPatterns,
} from "./generic-scanner-shared.mjs";

function addCommonViolation(context, lineNo, ruleId, detail, source) {
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

export function scanCommonFilePrelude(context) {
  const { rel, text } = context;
  if (GENERATED_PATH_RE.test(rel)) {
    if (!GENERATED_PROVENANCE_RE.test(text)) {
      addCommonViolation(context, 1, "GEN-2.2", "generated file lacks source owner or generator provenance.", rel);
    }
    if (/(?:contract|contracts)/iu.test(rel) && !GENERATED_HASH_RE.test(text)) {
      addCommonViolation(context, 1, "GEN-2.4", "generated contract artifact lacks source schema hash.", rel);
    }
    if (/(?:schema|schemas|\.json$)/iu.test(rel) && !GENERATED_HASH_RE.test(text)) {
      addCommonViolation(context, 1, "GEN-2.5", "generated schema artifact lacks reproducibility hash.", rel);
    }
    if (/\b(?:single source of truth|SOURCE_OF_TRUTH|authoritative)\b/iu.test(text)) {
      addCommonViolation(context, 1, "GEN-2.7", "generated file claims to be source of truth.", rel);
    }
  }
  if (
    /ocentra-enforcer\.config\.json|rust-rules\.config\.json/iu.test(rel)
    && /"importBoundaryPolicies"\s*:/u.test(text)
    && !/"architecturePolicyChecks"\s*:/u.test(text)
  ) {
    addCommonViolation(context, 1, "ARCH-1.10", "import boundary policy config lacks architecturePolicyChecks tests.", rel);
  }
  if (/(?:^|\/)(?:package\.json|Cargo\.toml)$/u.test(rel) && !context.hasOwnershipFile(path.dirname(context.filePath))) {
    addCommonViolation(context, 1, "ARCH-1.15", "package/crate has no OWNERS, CODEOWNERS, or ownership README.", rel);
  }
}

function scanCommandToolingLine(context, lineNo, line) {
  if (!isCommandLikeLine(line)) return;
  if (/\bgitleaks\s+(?:detect|protect|dir|git)\b/iu.test(line)) {
    if (!/\bsarif\b|--report-format\s+sarif/iu.test(line)) {
      addCommonViolation(context, lineNo, "SEC-2.16", "Gitleaks command does not emit SARIF.", line);
    }
    if (!/\bocentra-enforcer\b|--report-format\s+sarif/iu.test(line)) {
      addCommonViolation(context, lineNo, "SEC-2.17", "Gitleaks command is not normalized through Enforcer/SARIF.", line);
    }
  }
  if (/\btrufflehog\b/iu.test(line) && !/--json\b|\bjson\b|\bocentra-enforcer\b/iu.test(line)) {
    addCommonViolation(context, lineNo, "SEC-2.18", "TruffleHog command is not normalized through JSON/Enforcer.", line);
  }
  if (/\bruff\s+check\b/iu.test(line) && !/--output-format\s+json|--format\s+json|\bocentra-enforcer\b/iu.test(line)) {
    addCommonViolation(context, lineNo, "PY-5.5", "Ruff command does not emit JSON diagnostics.", line);
  }
  if (/\bpyright\b/iu.test(line) && !/--outputjson\b|\bocentra-enforcer\b/iu.test(line)) {
    addCommonViolation(context, lineNo, "PY-5.6", "Pyright command does not emit JSON diagnostics.", line);
  }
  if (/\bmypy\b/iu.test(line) && !/--junit-xml\b|--json-report\b|\bjson\b|\bocentra-enforcer\b/iu.test(line)) {
    addCommonViolation(context, lineNo, "PY-5.6", "mypy command does not emit structured diagnostics.", line);
  }
}

function scanSecretLine(context, lineNo, line) {
  if (OPENAI_KEY_RE.test(line)) {
    addCommonViolation(context, lineNo, "SEC-1.1", "OpenAI key found.", redactOpenAiKey(line));
  }
  if (SECRET_RE.test(line)) {
    addCommonViolation(context, lineNo, "SEC-1.1", "Inline secret-like assignment found.", redact(line));
  }
  for (const rule of COMMON_SECRET_RULES) {
    if (rule.pattern.test(line)) {
      addCommonViolation(context, lineNo, rule.ruleId, rule.detail, redact(line));
    }
  }
  if (/\.env\.(?:example|sample)$/iu.test(context.rel) && !ENV_PLACEHOLDER_ALLOWED.test(line) && SECRET_RE.test(line)) {
    addCommonViolation(context, lineNo, "SEC-2.11", ".env.example contains a real-looking secret.", redact(line));
  }
  if (/\.env\.template$/iu.test(context.rel) && !ENV_PLACEHOLDER_ALLOWED.test(line) && SECRET_RE.test(line)) {
    addCommonViolation(context, lineNo, "SEC-2.12", ".env.template contains a real-looking secret.", redact(line));
  }
  if (context.isTestLike && COMMON_SECRET_RULES.some((rule) => rule.pattern.test(line))) {
    addCommonViolation(context, lineNo, "SEC-2.13", "secret-looking value found in snapshot/test artifact.", redact(line));
  }
  if (/fixtures?\//iu.test(context.rel) && !/\bfake\b|\bfixture\b|\bexample\b/iu.test(line) && COMMON_SECRET_RULES.some((rule) => rule.pattern.test(line))) {
    addCommonViolation(context, lineNo, "SEC-2.14", "fixture secret lacks explicit fake marker.", redact(line));
  }
  if (SSH_KEY_PATH_RE.test(context.rel)) {
    addCommonViolation(context, lineNo, "SEC-2.19", "SSH key file found in source scope.", context.rel);
  }
  if (MOBILE_SECRET_CONFIG_RE.test(context.rel)) {
    addCommonViolation(context, lineNo, "SEC-2.20", "mobile secret config file found in source scope.", context.rel);
  }
}

function scanGeneratedLine(context, lineNo, line, comment) {
  if (
    isGeneratedMarkerSourceFile(context.rel, context.ext)
    && /@generated|<auto-generated>|Generated by/iu.test(comment)
    && !/\.d\.ts$/iu.test(context.rel)
  ) {
    addCommonViolation(context, lineNo, "GEN-1.1", "Generated artifact marker found in tracked source scope.", line);
    addCommonViolation(context, lineNo, "GEN-2.3", "generated file marker found; regenerate instead of editing manually.", line);
  }
  if (GENERATED_OUTPUT_DIR_RE.test(context.rel)) {
    addCommonViolation(context, lineNo, "GEN-2.6", "runtime output path is in source scope.", context.rel);
  }
  if (/(?:^|\/)generated(?:\/|$)/iu.test(context.rel)) {
    addCommonViolation(context, lineNo, "GEN-2.1", "generated directory file is in source scope.", context.rel);
    if (GENERATED_SUPPRESSION_RE.test(line)) {
      addCommonViolation(context, lineNo, "GEN-2.8", "generated code contains validation suppression.", line);
    }
  }
  if (/(?:@generated|<auto-generated>|Generated by)/iu.test(comment) && GENERATED_DOMAIN_PATH_RE.test(context.rel)) {
    addCommonViolation(context, lineNo, "GEN-2.9", "generated code is under a domain module path.", context.rel);
  }
  if (isGeneratedLikePath(context.rel) && GENERATED_SNAPSHOT_VOLATILE_RE.test(`${context.rel} ${line}`)) {
    addCommonViolation(context, lineNo, "GEN-2.10", "generated snapshot contains volatile value.", line);
  }
}

function scanTestLine(context, lineNo, line) {
  if (context.isTestLike) {
    for (const rule of weakAssertionPatterns) {
      if (rule.pattern.test(line)) {
        addCommonViolation(context, lineNo, "TEST-1.2", rule.detail, line);
      }
    }
  }
  if (context.ext === ".rs" && isTestPath(context.rel) && /#\s*\[\s*ignore\s*\]/u.test(line)) {
    addCommonViolation(context, lineNo, "TEST-1.3", "Rust #[ignore] test found.", line);
  }
}

function scanProductionPlaceholderLine(context, lineNo, line, comment) {
  if (!context.isProductionSource || context.isEnforcerTooling) return;
  for (const rule of placeholderDirectPatterns) {
    if (rule.pattern.test(line)) {
      addCommonViolation(context, lineNo, "SRC-1.2", rule.detail, line);
      addCommonViolation(context, lineNo, "SRC-2.10", rule.detail, line);
    }
  }
  if (comment !== "") {
    for (const rule of placeholderCommentPatterns) {
      if (rule.pattern.test(comment)) {
        addCommonViolation(context, lineNo, "SRC-1.2", rule.detail, line);
      }
    }
    for (const rule of [
      { pattern: /\btemporary\b/iu, detail: "temporary comment marker found." },
      { pattern: /\bfor now\b/iu, detail: "for now comment marker found." },
      { pattern: /\bhack\b/iu, detail: "hack comment marker found." },
      { pattern: /\bquick fix\b/iu, detail: "quick fix comment marker found." },
    ]) {
      if (rule.pattern.test(comment)) {
        addCommonViolation(context, lineNo, "SRC-2.9", rule.detail, line);
      }
    }
  }
  if (context.badSourceBasename) {
    addCommonViolation(context, lineNo, "SRC-2.8", "dumping-ground source filename found.", context.rel);
  }
}

export function scanCommonLineRules(context, line, index) {
  const lineNo = index + 1;
  const comment = sourceCommentText(context.ext, line);
  scanCommandToolingLine(context, lineNo, line);
  scanSecretLine(context, lineNo, line);
  scanGeneratedLine(context, lineNo, line, comment);
  scanTestLine(context, lineNo, line);
  scanProductionPlaceholderLine(context, lineNo, line, comment);
}

export function buildCommonScanContext(root, filePath, rel, ext, lines, text, violations, hasOwnershipFileFn) {
  return {
    root,
    filePath,
    rel,
    ext,
    lines,
    text,
    violations,
    hasOwnershipFile: hasOwnershipFileFn,
    isEnforcerTooling: isEnforcerToolingPath(rel),
    isTestLike: isTestPath(rel) || isPythonTestPath(rel),
    isProductionSource: isProductionSourcePath(rel, ext),
    badSourceBasename: BAD_SOURCE_BASENAME_RE.test(path.basename(rel)),
  };
}
