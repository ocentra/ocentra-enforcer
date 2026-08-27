import {
  addViolation,
  BOUNDARY_PATH_RE,
  countTextMatches,
  firstMatchingLine,
  isImportLikeLine,
} from "./generic-scanner-shared.mjs";
import { scanDomainAndArchitectureRules } from "./generic-common-source-ownership-architecture.mjs";

function addSourceOwnershipViolation(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

function hasBoundaryConversion(text) {
  return /\b(?:toDomain|fromRaw|parse|decode|validate)\w*\b|\bimpl\s+(?:TryFrom|From)\s*</u.test(text);
}

function hasSiblingBoundaryConversion(filePath, text) {
  const rawTypes = [...text.matchAll(/\b([A-Z][A-Za-z0-9_]*(?:Dto|DTO|Payload|Body|Request))\b/gu)]
    .map((match) => match[1]);
  if (rawTypes.length === 0) return false;
  const directory = path.dirname(filePath);
  const extension = path.extname(filePath);
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isFile() || entry.name === path.basename(filePath) || path.extname(entry.name) !== extension) continue;
    const sibling = fs.readFileSync(path.join(directory, entry.name), "utf8");
    if (hasBoundaryConversion(sibling) && rawTypes.some((name) => sibling.includes(name))) return true;
  }
  return false;
}

const BOUNDARY_DECISION_KEYWORD_RE = /\b(?:business|domain|role|plan|entitlement|policy)\b/iu;

function boundaryDecisionLine(lines) {
  for (let index = 0; index < lines.length; index += 1) {
    const decisionStart = lines[index].search(/\b(?:if|switch|match)\b/u);
    if (decisionStart < 0) continue;

    // A boundary may inspect a raw JSON field while decoding it.  The raw field
    // name is input vocabulary, not domain decision logic.  Inspect only the
    // decision header and remove quoted literals before looking for a domain
    // concept; the branch expression itself must carry that concept.
    const header = lines
      .slice(index, Math.min(index + 8, lines.length))
      .join("\n")
      .slice(decisionStart)
      .split("{")[0]
      .replace(/(?:r#*)?"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/gu, "")
      .replace(/\/\/.*$/gmu, "");
    if (BOUNDARY_DECISION_KEYWORD_RE.test(header)) return index + 1;
  }
  return 0;
}

function scanBoundaryRules(violations, root, filePath, rel, lines, text) {
  if (!BOUNDARY_PATH_RE.test(rel)) return;
  if (!/\bBOUNDARY-INVARIANT:/u.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "BOUND-1.1", "boundary file lacks BOUNDARY-INVARIANT documentation.", rel);
  }
  if (/\braw(?:Input|Dto|DTO|Payload|Body)?\b|:\s*(?:unknown|any|dict\[|Record<string,\s*unknown>)/u.test(text) && !hasBoundaryConversion(text) && !hasSiblingBoundaryConversion(filePath, text)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "BOUND-1.2", "raw boundary input is not converted to a domain type.", rel);
  }
  const decisionLine = boundaryDecisionLine(lines);
  if (decisionLine > 0) {
    addSourceOwnershipViolation(
      violations,
      root,
      filePath,
      decisionLine,
      "BOUND-1.3",
      "domain decision logic found in boundary file.",
      rel,
    );
  }
  if (!/\b(?:invalid|malformed|negative|reject|throws?|pytest\.raises)\b/iu.test(text)) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "BOUND-1.5", "boundary file lacks negative invalid-input coverage marker.", rel);
  }
  const rawTypeCount = countTextMatches(
    text,
    /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|type|interface|class)\s+(?:Raw[A-Z]\w+|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))\b/gm,
  );
  if (rawTypeCount > 3) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "BOUND-1.6", `boundary raw type count ${rawTypeCount} exceeds budget 3.`, rel);
  }
  if (/^(?:utils?|helpers?)\./iu.test(rel.split("/").at(-1) ?? "")) {
    addSourceOwnershipViolation(violations, root, filePath, 1, "BOUND-1.8", "boundary file uses utility/helper filename.", rel);
  }
  if (/\b(?:export\s+)?(?:function|const|def|fn)\s+\w+[^){]*\([^)]*(?:Dto|DTO|Payload|Raw|Request)[^)]*\)[^{;]*(?:Dto|DTO|Payload|Raw|Request)/u.test(text)) {
    addSourceOwnershipViolation(
      violations,
      root,
      filePath,
      firstMatchingLine(lines, /(?:Dto|DTO|Payload|Raw|Request)/u),
      "BOUND-1.9",
      "boundary DTO leaks into public/domain signature.",
      rel,
    );
  }
  if (/\b(?:toDomain|fromRaw|parse|decode|convert)\w*\s*\([^)]*\)\s*(?::|->)\s*(?:string|str|boolean|bool|void|unknown|any)\b/iu.test(text)) {
    addSourceOwnershipViolation(
      violations,
      root,
      filePath,
      firstMatchingLine(lines, /\b(?:toDomain|fromRaw|parse|decode|convert)/iu),
      "BOUND-1.10",
      "boundary conversion returns untyped primitive/error shape.",
      rel,
    );
  }
}

/** Scans one source file for boundary, domain, architecture, and ownership rules. */
export function scanSourceOwnershipPolicy(root, filePath, rel, lines) {
  const violations = [];
  const text = lines.join("\n");
  const importText = lines.filter(isImportLikeLine).join("\n");
  scanBoundaryRules(violations, root, filePath, rel, lines, text);
  scanDomainAndArchitectureRules(violations, root, filePath, rel, lines, text, importText);
  return violations;
}
import fs from "node:fs";
import path from "node:path";
