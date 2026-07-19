import { addViolation } from "./rust-rules-path-core.mjs";
import {
  NAME_PATTERNS,
  RAW_TYPE_PATTERNS,
} from "./rust-rules-source-patterns.mjs";
import {
  collectFunctionSignatures,
  functionName,
  functionParams,
} from "./rust-rules-source-helpers.mjs";
import { isTraitImplementationSignature } from "./rust-rules-source-signature-helpers.mjs";

const { FALLIBLE_FN_NAME_RE } = NAME_PATTERNS;
const { RAW_POINTER_RE, RAW_PRIMITIVE_TYPE_RE, RAW_STRING_TYPE_RE } =
  RAW_TYPE_PATTERNS;

const SIMPLE_SIGNATURE_RULES = [
  {
    ruleId: "RR-3.4",
    detail: "Raw pointer found in function signature.",
    matches: (sigText) => RAW_POINTER_RE.test(sigText),
  },
  {
    ruleId: "RR-4.7",
    detail: "Result uses String as the error type.",
    matches: (sigText) => /\bResult\s*<[^>]*,\s*String\s*>/u.test(sigText),
  },
  {
    ruleId: "RR-4.8",
    detail: "Result uses &'static str as the error type.",
    matches: (sigText) =>
      /\bResult\s*<[^>]*,\s*&\s*'static\s+str\s*>/u.test(sigText),
  },
  {
    ruleId: "RR-6.27",
    detail: "AsRef<str> found in domain function signature.",
    matches: (sigText) => /\bAsRef\s*<\s*str\s*>/u.test(sigText),
  },
  {
    ruleId: "RR-6.28",
    detail: "Into<String> found in domain function signature.",
    matches: (sigText) => /\bInto\s*<\s*String\s*>/u.test(sigText),
  },
  {
    ruleId: "RR-6.29",
    detail: "ID-like parameter accepts impl Display.",
    matches: (sigText) =>
      /\bimpl\s+Display\b/u.test(sigText) &&
      /\b(?:id|key|ref|name)\s*:/iu.test(sigText),
  },
  {
    ruleId: "RR-6.30",
    detail: "Cow<str> found in domain function signature.",
    matches: (sigText) => /\bCow\s*<[^>]*\bstr\b[^>]*>/u.test(sigText),
  },
  {
    ruleId: "RR-6.31",
    detail: "Vec<String> found in domain function signature.",
    matches: (sigText) => /\bVec\s*<\s*String\s*>/u.test(sigText),
  },
  {
    ruleId: "RR-6.32",
    detail: "HashMap<String, _> found in domain function signature.",
    matches: (sigText) => /\bHashMap\s*<\s*String\s*,/u.test(sigText),
  },
  {
    ruleId: "RR-6.33",
    detail: "BTreeMap<String, _> found in domain function signature.",
    matches: (sigText) => /\bBTreeMap\s*<\s*String\s*,/u.test(sigText),
  },
  {
    ruleId: "RR-6.34",
    detail: "serde_json::Value found in domain function signature.",
    matches: (sigText) => /\bserde_json::Value\b/u.test(sigText),
  },
  {
    ruleId: "RR-6.38",
    detail: "Raw Duration found in named domain timing parameter.",
    matches: (sigText) =>
      /\b(?:timeout|ttl|delay|interval|deadline|duration)\s*:\s*(?:std::time::)?Duration\b/iu.test(
        sigText,
      ),
  },
  {
    ruleId: "RR-6.39",
    detail: "Raw time type found in public domain signature.",
    matches: (sigText) => /\b(?:SystemTime|Instant)\b/u.test(sigText),
  },
  {
    ruleId: "RR-6.40",
    detail: "URL-like parameter uses raw string type.",
    matches: (sigText) =>
      /\b(?:url|uri|endpoint)\s*:\s*(?:String|&\s*str|str\b)/iu.test(sigText),
  },
  {
    ruleId: "RR-6.41",
    detail: "Path-like parameter uses raw string/path type.",
    matches: (sigText) =>
      /\b(?:path|file|dir|directory)\s*:\s*(?:String|&\s*str|str\b|PathBuf)/iu.test(
        sigText,
      ),
  },
  {
    ruleId: "RR-6.48",
    detail: "Naked tuple found in public/domain function signature.",
    matches: (sigText) =>
      /->\s*\([^)]*,[^)]*\)/u.test(sigText) ||
      /\([^)]*:\s*\([^)]*,[^)]*\)/u.test(sigText),
  },
  {
    ruleId: "RR-8.30",
    detail: "Raw Arc<Mutex<T>> appears in a function signature.",
    matches: (sigText) => /\bArc\s*<\s*(?:std::sync::)?Mutex\s*</u.test(sigText),
  },
];

function addSignatureViolation(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

function applySimpleSignatureRules({
  sigText,
  root,
  filePath,
  line,
  source,
  violations,
}) {
  for (const rule of SIMPLE_SIGNATURE_RULES) {
    if (!rule.matches(sigText)) continue;
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      rule.ruleId,
      rule.detail,
      source,
    );
  }
}

function applyFallibleSignatureRules({
  sigText,
  sigName,
  params,
  root,
  filePath,
  line,
  source,
  violations,
}) {
  if (FALLIBLE_FN_NAME_RE.test(sigName) && /->\s*bool\b/u.test(sigText)) {
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      "RR-4.12",
      "Fallible-looking API returns bool instead of Result or a status enum.",
      source,
    );
  }

  if (
    /\bfn\s+new\s*\(/u.test(sigText) &&
    /->\s*Self\b/u.test(sigText) &&
    (RAW_STRING_TYPE_RE.test(params) || RAW_PRIMITIVE_TYPE_RE.test(params)) &&
    !/Result\s*<\s*Self\s*,/u.test(sigText)
  ) {
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      "RR-4.14",
      "new(...) accepts raw input but does not return Result<Self, Error>.",
      source,
    );
  }
}

function applyOwnerSensitiveSignatureRules({
  sigText,
  sigName,
  params,
  root,
  filePath,
  line,
  source,
  violations,
  isStringOwner,
  isPrimitiveOwner,
  isBenchmark,
  isTraitImplementationSignature,
}) {
  if (isBenchmark || isTraitImplementationSignature) return;
  if (
    /\bfn\s+new\s*\(/u.test(sigText) &&
    (
      params.match(
        /\b(?:String|str|bool|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b/gu,
      ) ?? []
    ).length >= 2
  ) {
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      "RR-6.49",
      "Constructor accepts multiple primitive/raw parameters.",
      source,
    );
  }

  if (!isStringOwner && RAW_STRING_TYPE_RE.test(sigText)) {
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      "RR-6.1",
      "Raw string/path type found in function signature.",
      source,
    );
  }

  const normalizedParams = params.replace(/\s+/gu, " ").trim();
  const receiverOnly = /^(?:&\s*(?:'[_A-Za-z][_A-Za-z0-9]*\s+)?(?:mut\s+)?self|self)$/u.test(normalizedParams);
  const conventionalCollectionQuery = receiverOnly && (
    (sigName === "len" && /->\s*usize\b/u.test(sigText)) ||
    (sigName === "is_empty" && /->\s*bool\b/u.test(sigText))
  );
  if (!isPrimitiveOwner && !conventionalCollectionQuery && RAW_PRIMITIVE_TYPE_RE.test(sigText)) {
    addSignatureViolation(
      violations,
      root,
      filePath,
      line,
      "RR-6.2",
      "Unbranded primitive type found in function signature.",
      source,
    );
  }
}

export function applySignatureRules({
  masked,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isStringOwner,
  isPrimitiveOwner,
  isBenchmark,
}) {
  for (const sig of collectFunctionSignatures(masked)) {
    if (isBoundary) continue;

    const originalSigFirstLine = originalLines[sig.line - 1] ?? sig.text;
    const sigName = functionName(sig.text);
    const params = functionParams(sig.text);
    const traitImplementationSignature = isTraitImplementationSignature(masked, sig.index);

    applySimpleSignatureRules({
      sigText: sig.text,
      root,
      filePath,
      line: sig.line,
      source: originalSigFirstLine,
      violations,
    });
    applyFallibleSignatureRules({
      sigText: sig.text,
      sigName,
      params,
      root,
      filePath,
      line: sig.line,
      source: originalSigFirstLine,
      violations,
    });
    applyOwnerSensitiveSignatureRules({
      sigText: sig.text,
      sigName,
      params,
      root,
      filePath,
      line: sig.line,
      source: originalSigFirstLine,
      violations,
      isStringOwner,
      isPrimitiveOwner,
      isBenchmark,
      isTraitImplementationSignature: traitImplementationSignature,
    });
  }
}
