import {
  addViolation,
  BOUNDARY_PATH_RE,
  firstMatchingLine,
} from "./generic-scanner-shared.mjs";
import {
  decisionCodeText,
  declaredRawBoundaryTypeNames,
  hasCohesiveRustDtoFamily,
} from "./generic-common-domain-ownership.mjs";
import { hasNegativeBoundaryEvidence } from "./generic-common-boundary-test-evidence.mjs";
import { analyzeBoundarySignatures } from "./generic-common-boundary-signatures.mjs";

function record(violations, root, filePath, line, ruleId, detail, source) {
  addViolation(violations, root, filePath, line, ruleId, detail, source);
}

/** Scans one source file for boundary ownership contract violations. */
export function scanBoundaryRules(violations, root, filePath, rel, lines, text) {
  if (!BOUNDARY_PATH_RE.test(rel)) return;
  const rawTypes = declaredRawBoundaryTypeNames(text);
  const rawTypeCount = rawTypes.size;
  const signatureEvidence = analyzeBoundarySignatures(text, rawTypes);
  const isBoundarySurface = /\bBOUNDARY-INVARIANT:/u.test(text) || rawTypeCount > 0
    || /#\[derive\([^\]]*(?:Serialize|Deserialize)|\b(?:parse|decode|toDomain|fromRaw)\w*\s*\(/u.test(text);
  if (!isBoundarySurface) return;
  const documented = /\bBOUNDARY-INVARIANT:/u.test(text)
    || /(?:^|\n)\s*(?:\/\/[/!]|\/\*\*)[^\n]*(?:boundary|wire|raw)[\s\S]{0,600}(?:parse|decode|convert|map|domain|reject|validate)/iu.test(text);
  if (!documented) record(violations, root, filePath, 1, "BOUND-1.1", "boundary file lacks BOUNDARY-INVARIANT documentation.", rel);
  if ((rawTypeCount > 0 || /:\s*(?:unknown|any|dict\[|Record<string,\s*unknown>)/u.test(text))
      && !signatureEvidence.hasDomainConversion) {
    record(violations, root, filePath, 1, "BOUND-1.2", "raw boundary input is not converted to a domain type.", rel);
  }
  const decisionText = decisionCodeText(lines);
  if (/\b(?:if|switch|match)\b[\s\S]{0,120}\b(?:business|domain|role|plan|entitlement|policy)\b/iu.test(decisionText)) {
    record(violations, root, filePath, firstMatchingLine(lines, /\b(?:business|domain|role|plan|entitlement|policy)\b/iu), "BOUND-1.3", "domain decision logic found in boundary file.", rel);
  }
  const transformsRawBoundaryInput = rawTypeCount > 0
    && /\b(?:decode|parse|validate|try_from|toDomain|fromRaw|convert)\w*\s*\(/iu.test(text);
  if (transformsRawBoundaryInput && !hasNegativeBoundaryEvidence(root, rel, text, rawTypes)) record(violations, root, filePath, 1, "BOUND-1.5", "boundary decoder or converter lacks negative invalid-input coverage.", rel);
  const cohesive = hasCohesiveRustDtoFamily(rel, text, rawTypes);
  if (rawTypeCount > 3 && !cohesive) record(violations, root, filePath, 1, "BOUND-1.6", `boundary raw type count ${rawTypeCount} exceeds budget 3.`, rel);
  if (/^(?:utils?|helpers?)\./iu.test(rel.split("/").at(-1) ?? "")) record(violations, root, filePath, 1, "BOUND-1.8", "boundary file uses utility/helper filename.", rel);
  if (signatureEvidence.publicRawReturn) {
    record(violations, root, filePath, firstMatchingLine(lines, /(?:Dto|DTO|Payload|Raw|Request)/u), "BOUND-1.9", "boundary DTO leaks through a public boundary signature.", rel);
  }
  if (signatureEvidence.untypedConversion || signatureEvidence.untypedTryFromError) {
    record(violations, root, filePath, firstMatchingLine(lines, /\b(?:toDomain|fromRaw|parse|decode|convert)/iu), "BOUND-1.10", "boundary conversion returns untyped primitive/error shape.", rel);
  }
}
