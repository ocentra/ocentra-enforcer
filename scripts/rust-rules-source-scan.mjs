#!/usr/bin/env node
/*
 * Ocentra Enforcer Rust source scan engine.
 */
import fs from "node:fs";
import path from "node:path";
import {
  normalizeRel,
  matchesAnyGlob,
  lineNumberAt,
  maskRustCode,
  contextHas,
  firstLineMatching,
  escapeRegExp,
  lineNumberAtIndex,
  addViolation,
} from "./rust-rules-path-core.mjs";
import {
  NAME_PATTERNS,
  RAW_TYPE_PATTERNS,
  STRUCT_FIELD_PATTERNS,
} from "./rust-rules-source-patterns.mjs";
import {
  collectFunctionSignatures,
  functionName,
  functionParams,
  normalizedNameTokens,
  isSuspiciousSerializedFieldName,
  braceDelta,
  hasStringLiteral,
} from "./rust-rules-source-helpers.mjs";
import {
  isTestFile,
  isRawTypeBoundary,
  isBoundaryModulePath,
  isRawStringOwner,
  isDomainPrimitiveOwner,
  isRuntimeStringOwner,
  isSerializedDomainOwner,
} from "./rust-rules-source-classification.mjs";
import { applyLateRustFileRules } from "./rust-rules-source-late-rules.mjs";
import { applySignatureRules } from "./rust-rules-source-signature-rules.mjs";

const { ID_LIKE_NAME_RE, PATH_LIKE_NAME_RE, TIME_LIKE_NAME_RE, URL_LIKE_NAME_RE } =
  NAME_PATTERNS;
const { RAW_POINTER_RE, RAW_PRIMITIVE_TYPE_RE, RAW_STRING_TYPE_RE, TYPE_ALIAS_RAW_RE } =
  RAW_TYPE_PATTERNS;
const { FIELD_RE, PUBLIC_FIELD_RE, PUBLIC_SERDE_STRUCT_RE } =
  STRUCT_FIELD_PATTERNS;

// boundaryOwnerNote: Enforcer-owned Rust scan engine; edits require policy-integrity and self-scan validation.
// Contract rule markers retained on the public orchestrator for registry coverage:
// "RR-3.4" "RR-4.7" "RR-4.8" "RR-4.12" "RR-4.14" "RR-6.1" "RR-6.2"
// "RR-3.30" "RR-3.31" "RR-6.44" "RR-6.50"
// "RR-6.27" "RR-6.28" "RR-6.29" "RR-6.30" "RR-6.31" "RR-6.32" "RR-6.33"
// "RR-6.34" "RR-6.38" "RR-6.39" "RR-6.40" "RR-6.41" "RR-6.48" "RR-6.49"
// "RR-12.16" "RR-12.17" "RR-12.18" "RR-12.19" "RR-12.20"
// "RR-12.24" "RR-12.25" "RR-12.26" "RR-12.27" "RR-12.28" "RR-12.29" "RR-12.30"
// "RR-14.20" "RR-14.21" "RR-14.23" "RR-14.24" "RR-14.25" "RR-14.26" "RR-14.28"
// "RR-8.30"

function scanRustFile(root, filePath, config) {
  const rel = normalizeRel(root, filePath);
  const violations = [];
  const source = fs.readFileSync(filePath, "utf8");
  const masked = maskRustCode(source);
  const originalLines = source.split(/\r?\n/u);
  const maskedLines = masked.split(/\r?\n/u);
  const isTestSource = isTestFile(rel, config);
  const isBoundary = isBoundaryModulePath(rel, config);
  const isStringOwner = isRawStringOwner(rel, config);
  const isPrimitiveOwner = isDomainPrimitiveOwner(rel, config);
  const enforceRuntimeStrings =
    config.enforceRuntimeStringLiterals &&
    !isTestSource &&
    !isRuntimeStringOwner(rel, config);
  const enforceSerializedDomainFields =
    config.enforceSerializedPublicDomainPrimitives &&
    !isTestSource &&
    !isSerializedDomainOwner(rel, config);
  const fileName = path.basename(filePath);
  const badModuleFileNames = new Set([
    "utils.rs",
    "helper.rs",
    "helpers.rs",
    "common.rs",
    "misc.rs",
    "stuff.rs",
    "shared.rs",
  ]);

  if (badModuleFileNames.has(fileName)) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      "RR-7.4",
      `Forbidden dumping-ground file name: ${fileName}.`,
    );
  }

  let pendingSerializeDerive = false;
  let pendingSerdeShape = false;
  let trackedSerdeStructDepth = 0;

  maskedLines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const originalLine = originalLines[idx] ?? line;

    if (
      /^\s*#!?\s*\[\s*(?:allow|expect)\s*\(/u.test(line) ||
      /^\s*#!?\s*\[\s*cfg_attr\s*\([^\]]*\b(?:allow|expect)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.1",
        "Lint suppression attribute found.",
        originalLine,
      );
      if (/\bunsafe_code\b/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.33",
          "allow(unsafe_code) suppression found.",
          originalLine,
        );
      }
    }

    if (/\brustfmt::skip\b|\bclippy::(?:allow|expect)\b/u.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.1",
        "Rust formatter or Clippy suppression found.",
        originalLine,
      );
    }

    if (/rust-rules\s*:\s*(?:ignore|allow|skip|disable)/iu.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-2.2",
        "Validator suppression comment found.",
        originalLine,
      );
    }

    if (/\bunsafe\b/u.test(line)) {
      if (isTestSource) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.32",
          "unsafe code found in test source.",
          originalLine,
        );
      }
      if (!config.allowUnsafeCode) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.1",
          "unsafe keyword found while allowUnsafeCode=false.",
          originalLine,
        );
      } else if (
        /\bunsafe\s*\{/u.test(line) &&
        !contextHas(originalLines, idx, "SAFETY:", 4)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.2",
          "unsafe block lacks nearby // SAFETY: comment.",
          originalLine,
        );
      } else if (
        /\bunsafe\s+fn\b/u.test(line) &&
        !contextHas(originalLines, idx, "# Safety", 8)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-3.3",
          "unsafe fn lacks rustdoc # Safety section.",
          originalLine,
        );
      }
    }

    if (/\b(?:core|std)::mem::transmute\b|\btransmute\s*(?:::<[^>]+>)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.16",
        "transmute found in Rust source.",
        originalLine,
      );
    }

    if (/\bMaybeUninit\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.17",
        "MaybeUninit found outside an approved unsafe owner.",
        originalLine,
      );
    }

    if (/\bManuallyDrop\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.18",
        "ManuallyDrop found without reviewed unsafe invariants.",
        originalLine,
      );
    }

    if (/\b(?:core|std)::mem::forget\b|\bmem::forget\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.19",
        "mem::forget found.",
        originalLine,
      );
    }

    if (/\bBox::leak\s*\(/u.test(line) && !contextHas(originalLines, idx, "LEAK-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.20",
        "Box::leak lacks LEAK-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/^\s*(?:pub\s+)?static\s+mut\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.21",
        "static mut found in Rust source.",
        originalLine,
      );
    }

    if (/\bUnsafeCell\b/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.22",
        "UnsafeCell found outside an approved primitive.",
        originalLine,
      );
    }

    if (/^\s*unsafe\s+impl\s+(?:Send|Sync)\b/u.test(line) && !contextHas(originalLines, idx, "SAFETY:", 6)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.23",
        "unsafe Send/Sync impl lacks nearby SAFETY proof.",
        originalLine,
      );
    }

    if (/\bget_unchecked(?:_mut)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.24",
        "get_unchecked found.",
        originalLine,
      );
    }

    if (/\bunsafe\s*\{[^}]*\*[A-Za-z_][A-Za-z0-9_]*/u.test(line) && !contextHas(originalLines, idx, "SAFETY:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.25",
        "raw pointer dereference lacks nearby SAFETY proof.",
        originalLine,
      );
    }

    if (/^\s*(?:pub\s+)?extern\s+(?:"[^"]+"\s*)?\{/u.test(line) && !/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.26",
        "extern block found outside ffi/sys module.",
        originalLine,
      );
    }

    if (/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel) && /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\b/u.test(line) && !contextHas(originalLines, idx, "repr(C)", 2)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.27",
        "FFI-facing public struct lacks #[repr(C)].",
        originalLine,
      );
    }

    if (/^\s*#\s*\[\s*no_mangle\s*\]/u.test(line) && !/(?:^|\/)(?:ffi|sys)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.28",
        "#[no_mangle] found outside ffi/sys module.",
        originalLine,
      );
    }

    if ((/^\s*#\s*\[\s*no_mangle\s*\]/u.test(line) || /\bextern\s+"C"\s+fn\b/u.test(line)) && !contextHas(originalLines, idx, "catch_unwind", 12) && !contextHas(originalLines, idx, "PANIC-ABORT", 12)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-3.29",
        "FFI export lacks catch_unwind or PANIC-ABORT evidence.",
        originalLine,
      );
    }

    if (/\.unwrap\s*\(|\.expect\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.1",
        ".unwrap() or .expect() found.",
        originalLine,
      );
    }

    if (!isBoundary && /\bErr\s*\(/u.test(line) && /\bErr\s*\(\s*(?:b?"|b?r#*")/u.test(originalLine)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.9",
        "Literal string error found.",
        originalLine,
      );
    }

    if (!isBoundary && /\bErr\s*\(\s*format\s*!/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.10",
        "Formatted string error found.",
        originalLine,
      );
    }

    if (!isBoundary && /\.map_err[^\n]*\.to_string\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.11",
        "map_err erases the source error into String.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*let\s+_\s*=\s*[^;]*(?:send|write|flush|read|parse|save|load|remove|create|open|connect|try_[A-Za-z0-9_]*)\s*(?:::<[^>]+>)?\s*\(/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.16",
        "Fallible-looking result is ignored with let _ = ...;",
        originalLine,
      );
    }

    if (!isBoundary && /\.ok\s*\(\s*\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.17",
        "Result::ok() swallows the typed error.",
        originalLine,
      );
    }

    if (!isBoundary && /\.unwrap_or_default\s*\(\s*\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.18",
        "unwrap_or_default hides a fallible domain/config value.",
        originalLine,
      );
    }

    if (/\b(?:unwrap|expect|panic)\s*(?:!|\()/u.test(line) && contextHas(originalLines, idx, "fn new", 12)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.45",
        "newtype constructor panics or unwraps validation.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /\.(?:unwrap_or|unwrap_or_else)\s*\(/u.test(line) &&
      /\b(?:parse|env|config|read|load|decode|deserialize)\b/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.19",
        "unwrap_or hides a parse/config/domain failure.",
        originalLine,
      );
    }

    if (/\b(?:panic|todo|unimplemented|unreachable)\s*!\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.2",
        "panic-like macro found.",
        originalLine,
      );
    }

    if (isTestFile(rel, config) && /\.unwrap\s*\(|\.expect\s*\(/u.test(line) && !contextHas(originalLines, idx, "TEST-UNWRAP-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.21",
        "test unwrap/expect lacks TEST-UNWRAP-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/\b(?:dbg|println|eprintln)\s*!\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.3",
        "debug/console macro found.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /\banyhow::Result\b|\bBox\s*<\s*dyn\s+(?:std::error::Error|Error)\b/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.4",
        "Erased application error type found in non-boundary code.",
        originalLine,
      );
    }

    if (
      /\.clone\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "CLONE-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.1",
        ".clone() found without nearby CLONE-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\.(?:to_string|to_owned)\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "ALLOC-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.2",
        "String allocation found without nearby ALLOC-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\b[A-Za-z_][A-Za-z0-9_\.]*\s*\[[^\]\n]+\]/u.test(line) &&
      !/\b(?:vec|format|println|assert|assert_eq|assert_ne)!\s*\[/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.3",
        "Unchecked indexing/slicing found.",
        originalLine,
      );
    }

    if (
      /\s+as\s+(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u.test(
        line,
      ) &&
      !contextHas(originalLines, idx, "CAST-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-5.4",
        "Numeric cast found without nearby CAST-JUSTIFICATION.",
        originalLine,
      );
    }

    const typeAliasMatch = line.match(TYPE_ALIAS_RAW_RE);
    if (
      typeAliasMatch &&
      (RAW_STRING_TYPE_RE.test(typeAliasMatch[1]) ||
        RAW_PRIMITIVE_TYPE_RE.test(typeAliasMatch[1]))
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.5",
        "Raw primitive/string type alias found.",
        originalLine,
      );
    }

    if (/^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+[A-Z][A-Za-z0-9_]*\s*=\s*(?:Vec|HashMap|BTreeMap)\s*</u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.47",
        "Domain collection type alias found.",
        originalLine,
      );
    }

    if (
      /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\(\s*pub\s+/u.test(line) &&
      (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.43",
        "Public tuple newtype exposes raw inner field.",
        originalLine,
      );
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.6",
        "Public tuple newtype exposes raw inner field.",
        originalLine,
      );
    }

    const tupleNewtypeMatch = line.match(/^\s*pub\s+struct\s+([A-Z][A-Za-z0-9_]*(?:Id|ID|Key|Ref))\s*\(\s*(?:pub\s+)?(?<type>u8|u16|u32|u64|u128|usize)\s*\)\s*;/u);
    if (tupleNewtypeMatch) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.46",
        "Numeric ID newtype uses a raw integer instead of NonZero or a validated representation.",
        originalLine,
      );
    }

    if (/^\s*#\[derive\([^#\]]*\bDebug\b[^#\]]*\)\]/u.test(line)) {
      const nextLine =
        maskedLines
          .slice(idx + 1, idx + 4)
          .find((candidate) => candidate.trim() !== "" && !/^\s*#\[/u.test(candidate)) ?? "";
      if (/(?:Secret|Token|Key|Credential|Password)/u.test(nextLine)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.51",
          "Secret-like type derives Debug.",
          originalLine,
        );
      }
    }

    if (/(?:Secret|Token|Key|Credential|Password)/u.test(line) && /\b(?:struct|enum)\b/u.test(line) && !/\bRedacted\b|REDACTED|redact/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.52",
        "secret-like type lacks redacted formatting evidence.",
        originalLine,
      );
    }

    if (
      /^\s*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*/u.test(
        line,
      ) &&
      (RAW_STRING_TYPE_RE.test(line) || RAW_PRIMITIVE_TYPE_RE.test(line))
    ) {
      if (/^\s*pub(?:\([^)]*\))?\s+/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.3",
          "Public raw field found.",
          originalLine,
        );
      } else if (
        !isBoundary &&
        !contextHas(originalLines, idx, "BRAND-INVARIANT:", 6)
      ) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.4",
          "Private raw field lacks nearby BRAND-INVARIANT documentation.",
          originalLine,
        );
      }
    }

    const fieldMatch = line.match(FIELD_RE);
    if (!isBoundary && fieldMatch?.groups) {
      const fieldName = fieldMatch.groups.name;
      const fieldType = fieldMatch.groups.type;
      if (/\bBTreeMap\s*<\s*String\s*,/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.33",
          "BTreeMap<String, _> found in domain field.",
          originalLine,
        );
      }
      if (/\bserde_json::Value\b|\bValue\b/u.test(fieldType) && /serde_json|json|value/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.34",
          "serde_json::Value found in domain field.",
          originalLine,
        );
      }
      if (/\bOption\s*<\s*String\s*>/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.35",
          "Option<String> found in domain field.",
          originalLine,
        );
      }
      if (/\bOption\s*<\s*bool\s*>/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.36",
          "Option<bool> found in domain field.",
          originalLine,
        );
      }
      if (TIME_LIKE_NAME_RE.test(fieldName) && /\bDuration\b/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.38",
          "Raw Duration found in named domain timing field.",
          originalLine,
        );
      }
      if (/\b(?:SystemTime|Instant)\b/u.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.39",
          "Raw time type found in domain field.",
          originalLine,
        );
      }
      if (URL_LIKE_NAME_RE.test(fieldName) && RAW_STRING_TYPE_RE.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.40",
          "URL-like field uses a raw string type.",
          originalLine,
        );
      }
      if (PATH_LIKE_NAME_RE.test(fieldName) && RAW_STRING_TYPE_RE.test(fieldType)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.41",
          "Path-like field uses a raw string type.",
          originalLine,
        );
      }
      if (ID_LIKE_NAME_RE.test(fieldName) && (RAW_STRING_TYPE_RE.test(fieldType) || RAW_PRIMITIVE_TYPE_RE.test(fieldType))) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-6.42",
          "ID-like field uses a raw string or primitive type.",
          originalLine,
        );
      }
    }

    if (
      /^\s*pub\s+struct\s+[A-Z][A-Za-z0-9_]*\s*\([^)]*(?:String|str|bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)/u.test(
        line,
      ) &&
      !contextHas(originalLines, idx, "BRAND-INVARIANT:", 6)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-6.4",
        "Tuple newtype over raw field lacks BRAND-INVARIANT documentation.",
        originalLine,
      );
    }

    if (/^\s*use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.1",
        "Wildcard import found.",
        originalLine,
      );
    }

    if (/^\s*pub\s+use\s+[^;]*::\*\s*;/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.2",
        "Wildcard public re-export found.",
        originalLine,
      );
    }

    if (/^\s*pub\s+use\s+/u.test(line)) {
      const isFacade = matchesAnyGlob(rel, config.facadeFileGlobs);
      if (config.publicReexportPolicy === "forbid") {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-7.3",
          "pub use is forbidden by this profile.",
          originalLine,
        );
      } else if (config.publicReexportPolicy === "facade-only" && !isFacade) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-7.3",
          "pub use outside configured facade file.",
          originalLine,
        );
      }
    }

    if (
      /^\s*(?:pub\s+)?mod\s+(?:utils|helper|helpers|common|misc|stuff|shared)\s*;/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-7.4",
        "Forbidden dumping-ground module declaration.",
        originalLine,
      );
    }

    if (
      /\basync\s+fn\b|\.await\b/u.test(masked) &&
      /\bstd::sync::(?:Mutex|RwLock)\b|\bstd::thread::sleep\b|\bstd::fs::|\bstd::net::/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.1",
        "Blocking primitive in async module.",
        originalLine,
      );
      if (/\bstd::sync::(?:Mutex|RwLock)\b/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.16",
          "std sync lock found in async module.",
          originalLine,
        );
      }
      if (/\bstd::fs::|\bstd::net::/u.test(line)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.25",
          "Blocking std I/O found in async module.",
          originalLine,
        );
      }
    }

    if (
      /\btokio::spawn\s*\(/u.test(line) &&
      !/[A-Za-z_][A-Za-z0-9_]*\s*=\s*tokio::spawn\s*\(/u.test(line) &&
      !/\b(?:let|return)\b/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.18",
        "tokio::spawn handle is not tracked.",
        originalLine,
      );
      if (!contextHas(originalLines, idx, "TASK-JUSTIFICATION:", 4)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-8.19",
          "fire-and-forget tokio::spawn lacks TASK-JUSTIFICATION.",
          originalLine,
        );
      }
    }

    if (/\.await\b/u.test(line) && /\b(?:MutexGuard|RwLock.*Guard|\.lock\s*\(\s*\)|\.write\s*\(\s*\)|\.read\s*\(\s*\))/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.17",
        "await appears while a lock guard is held.",
        originalLine,
      );
    }

    if (/\b(?:retry|retries|Retry)\b/u.test(line) && /\b(?:loop|while|for)\b/u.test(line) && !/\bRetryPolicy\b|BACKOFF-JUSTIFICATION|RETRY-JUSTIFICATION/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.22",
        "retry loop lacks bounded retry policy.",
        originalLine,
      );
    }

    if (/\bselect!\s*\{/u.test(line) && !contextHas(originalLines, idx, "CANCEL-SAFE:", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.24",
        "select! lacks nearby CANCEL-SAFE branch documentation.",
        originalLine,
      );
    }

    if ((/\basync\s+fn\b|\.await\b/u.test(masked)) && /\b(?:for|while)\b/u.test(line) && /\b(?:hash|compress|encode|decode|sort|parse|render|compute)\b/iu.test(line) && !/\bspawn_blocking\b|CPU-JUSTIFICATION|worker/u.test(source)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.26",
        "CPU-looking work in async path lacks spawn_blocking/worker boundary.",
        originalLine,
      );
    }

    if (
      /\b(?:tokio::sync::mpsc::|mpsc::)?unbounded_channel\s*(?:::<[^>]+>)?\s*\(/u.test(line) &&
      !contextHas(originalLines, idx, "CHANNEL-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.20",
        "Unbounded channel lacks CHANNEL-JUSTIFICATION.",
        originalLine,
      );
    }

    if (
      /\.(?:send|get|post|put|patch|delete|request)\s*\([^;\n]*\)\s*\.await\b/u.test(line) &&
      !/(?:timeout|Timeout|deadline)/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.21",
        "external async I/O await lacks timeout policy.",
        originalLine,
      );
    }

    if (/^\s*loop\s*\{/u.test(line) && /\basync\s+fn\b|\.await\b/u.test(masked) && !contextHas(originalLines, idx, "CANCEL", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.23",
        "async loop lacks nearby cancellation marker.",
        originalLine,
      );
    }

    if (/\b(?:tokio::runtime::Runtime::new|tokio::runtime::Builder::new)/u.test(line) && !/(?:^|\/)(?:main|bin)(?:\.rs|\/)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.27",
        "library/domain source creates a Tokio runtime.",
        originalLine,
      );
    }

    if (/\bblock_on\s*\(/u.test(line) && !/(?:^|\/)(?:main|bin)(?:\.rs|\/)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.28",
        "block_on found in library/domain source.",
        originalLine,
      );
    }

    if (isTestFile(rel, config) && /\b(?:std::thread::sleep|tokio::time::sleep)\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.29",
        "sleep found in test source.",
        originalLine,
      );
    }

    if (/\bfor\s+[A-Za-z_][A-Za-z0-9_]*\s+in\s+0\s*\.\./u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-8.2",
        "C-style numeric loop found.",
        originalLine,
      );
    }

    if (
      enforceRuntimeStrings &&
      hasStringLiteral(originalLine) &&
      !config.runtimeStringLineAllowRegexps.some((pattern) =>
        pattern.test(originalLine),
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-18.16",
        "Runtime Rust source contains an inline string literal.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*#\[derive\([^#\]]*\bDeserialize\b[^#\]]*\)\]/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.16",
        "Deserialize derive found in non-boundary Rust domain source.",
        originalLine,
      );
    }

    if (!isBoundary && /^\s*#\[derive\([^#\]]*\bSerialize\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "SERIALIZATION-DOC:", 8)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.17",
        "Serialize derive lacks SERIALIZATION-DOC evidence.",
        originalLine,
      );
    }

    if (/^\s*#\[serde\([^#\]]*\bdefault\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "DEFAULT-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.19",
        "serde(default) lacks DEFAULT-JUSTIFICATION.",
        originalLine,
      );
    }

    if (/^\s*#\[serde\([^#\]]*\bflatten\b[^#\]]*\)\]/u.test(line) && !contextHas(originalLines, idx, "FLATTEN-JUSTIFICATION:", 4)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.27",
        "serde(flatten) lacks FLATTEN-JUSTIFICATION.",
        originalLine,
      );
    }

    if (!isBoundary && /\bserde_json::from_str\s*(?:::<[^>]+>)?\s*\(/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.29",
        "serde_json::from_str found outside boundary source.",
        originalLine,
      );
    }

    if (!isBoundary && /\bserde_json::from_str\s*::?\s*<\s*[A-Z][A-Za-z0-9_]*(?:Domain|Id|State|Config|Policy|Record)?\s*>/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.30",
        "JSON deserializes directly into a domain-like type.",
        originalLine,
      );
    }

    if (/\buse\s+[^;]*(?:dto|request|response|envelope|transport|serde)[^;]*;/iu.test(line) && /(?:^|\/)(?:domain|core|model|models)(?:\/|$)/u.test(rel)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.22",
        "domain module imports DTO/transport/serde module.",
        originalLine,
      );
    }

    if (
      !isBoundary &&
      /^\s*#\[serde\([^#\]]*\buntagged\b[^#\]]*\)\]/u.test(line) &&
      !contextHas(originalLines, idx, "SERDE-UNTAGGED-JUSTIFICATION:", 4)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.18",
        "serde untagged enum lacks SERDE-UNTAGGED-JUSTIFICATION.",
        originalLine,
      );
    }

    if (enforceSerializedDomainFields) {
      if (trackedSerdeStructDepth === 0) {
        if (
          /^\s*#\[derive\([^#\]]*\b(?:Serialize|Deserialize)\b[^#\]]*\)\]/u.test(
            line,
          )
        ) {
        pendingSerializeDerive = true;
          if (!isBoundary && /\bDeserialize\b/u.test(line)) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "RR-14.16",
              "Deserialize derive found in non-boundary Rust domain source.",
              originalLine,
            );
          }
          return;
        }
        if (/^\s*#\[serde\(/u.test(line)) {
          if (
            !isBoundary &&
            /\buntagged\b/u.test(line) &&
            !contextHas(originalLines, idx, "SERDE-UNTAGGED-JUSTIFICATION:", 4)
          ) {
            addViolation(
              violations,
              root,
              filePath,
              lineNo,
              "RR-14.18",
              "serde untagged enum lacks SERDE-UNTAGGED-JUSTIFICATION.",
              originalLine,
            );
          }
          pendingSerdeShape = true;
          return;
        }
        if (pendingSerializeDerive || pendingSerdeShape) {
          if (/^\s*#\[/u.test(line)) return;
          const shouldTrack =
            pendingSerializeDerive &&
            pendingSerdeShape &&
            PUBLIC_SERDE_STRUCT_RE.test(line);
          pendingSerializeDerive = false;
          pendingSerdeShape = false;
          if (shouldTrack) trackedSerdeStructDepth = braceDelta(line);
          return;
        }
      } else {
        const match = PUBLIC_FIELD_RE.exec(line);
        if (
          match &&
          isSuspiciousSerializedFieldName(match.groups.name) &&
          (RAW_STRING_TYPE_RE.test(match.groups.type) ||
            RAW_PRIMITIVE_TYPE_RE.test(match.groups.type))
        ) {
          addViolation(
            violations,
            root,
            filePath,
            lineNo,
            "RR-6.26",
            "Serialized public struct field uses raw id/ref/event/command primitive outside configured owner crates.",
            originalLine,
          );
        }
        trackedSerdeStructDepth += braceDelta(line);
      }
    }

    if (/\bassert!\s*\(.*\.is_ok\s*\(\s*\).*?\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.22",
        "Weak is_ok assertion found.",
        originalLine,
      );
    }

    if (/\bassert!\s*\(.*\.is_some\s*\(\s*\).*?\)/u.test(line)) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-12.23",
        "Weak is_some assertion found.",
        originalLine,
      );
    }
  });

  if (!isBoundary) {
    for (const match of source.matchAll(/\bstruct\s+([A-Z][A-Za-z0-9_]*)[^{;]*\{([\s\S]*?)^\s*\}/gmu)) {
      const boolFields = [...match[2].matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*bool\s*,?/gmu)];
      if (boolFields.length >= 2) {
        addViolation(
          violations,
          root,
          filePath,
          lineNumberAtIndex(source, match.index),
          "RR-6.37",
          `Struct ${match[1]} has ${boolFields.length} boolean state fields.`,
          originalLines[lineNumberAtIndex(source, match.index) - 1] ?? null,
        );
      }
    }

    for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*(?:pub\s+)?enum\s+(?<name>[A-Z][A-Za-z0-9_]*Error)\b[^{]*\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const lineNo = lineNumberAtIndex(source, match.index);
      const attrs = match.groups?.attrs ?? "";
      const body = match.groups?.body ?? "";
      if (!/\bDebug\b/u.test(attrs) || !/\b(?:thiserror::)?Error\b/u.test(attrs)) {
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.20",
          `Error enum ${match.groups?.name ?? "Error"} does not derive Debug and Error.`,
          originalLines[lineNo - 1] ?? null,
        );
      }
      for (const variant of body.matchAll(/^\s*(?<attrs>(?:#\[[^\]]+\]\s*)*)[A-Z][A-Za-z0-9_]*\s*\(\s*(?<type>(?:std::)?[A-Za-z_:]+Error)\s*\)/gmu)) {
        if (!/\b(?:source|from)\b/u.test(variant.groups?.attrs ?? "")) {
          addViolation(
            violations,
            root,
            filePath,
            lineNumberAtIndex(source, (match.index ?? 0) + variant.index),
            "RR-4.21",
            "Wrapped source error lacks #[source] or #[from].",
            variant[0],
          );
        }
      }
    }

    for (const match of masked.matchAll(/\bfn\s+(?<name>find|get|lookup|parse)[A-Za-z0-9_]*\b[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const body = match.groups?.body ?? "";
      if (/\breturn\s+(?:-1|""|None)\s*;|=>\s*(?:-1|"")\b/u.test(body)) {
        const lineNo = lineNumberAt(masked, match.index);
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.13",
          "Lookup/parse function returns a sentinel failure value.",
          originalLines[lineNo - 1] ?? null,
        );
      }
    }

    for (const match of masked.matchAll(/\bfn\s+[A-Za-z_][A-Za-z0-9_]*\b[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
      const body = match.groups?.body ?? "";
      if (/\b(?:error|warn)!\s*\(/u.test(body) && /\bErr\s*\(/u.test(body)) {
        const lineNo = lineNumberAt(masked, match.index);
        addViolation(
          violations,
          root,
          filePath,
          lineNo,
          "RR-4.22",
          "Function logs and returns an error.",
          originalLines[lineNo - 1] ?? null,
        );
      }
    }

    const mainMatch = masked.match(/\bfn\s+main\s*\([^)]*\)\s*(?!->)[\s\S]*?\{(?<body>[\s\S]*?)^\s*\}/mu);
    if (mainMatch && /\blet\s+_\s*=\s*[A-Za-z_][A-Za-z0-9_]*(?:::[^(\s]+)?\s*\(/u.test(mainMatch.groups?.body ?? "")) {
      const lineNo = lineNumberAt(masked, mainMatch.index ?? 0);
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-4.15",
        "main swallows a fallible-looking call with let _ =.",
        originalLines[lineNo - 1] ?? null,
      );
    }
  }

  applySignatureRules({
    masked,
    originalLines,
    root,
    filePath,
    violations,
    isBoundary,
    isStringOwner,
    isPrimitiveOwner,
  });

  applyLateRustFileRules({
    source,
    masked,
    originalLines,
    root,
    filePath,
    violations,
    isBoundary,
    isTestSource,
  });

  return violations;
}

export { scanRustFile };
