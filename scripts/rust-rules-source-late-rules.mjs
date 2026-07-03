import { applyBoundaryTransportRules } from "./rust-rules-source-late-boundaries.mjs";
import {
  applyDomainBase64Rules,
  applyDomainDebugRules,
} from "./rust-rules-source-late-domain-debug.mjs";
import { applyProofEvidenceRules } from "./rust-rules-source-late-test-evidence.mjs";
import { applyTestStructureRules } from "./rust-rules-source-late-test-structure.mjs";
import {
  applyNewtypeConstructorRules,
  applyUnsafeEvidenceRules,
} from "./rust-rules-source-late-unsafe.mjs";

export function applyLateRustFileRules(context) {
  applyUnsafeEvidenceRules(context);
  applyNewtypeConstructorRules(context);
  applyDomainDebugRules(context);
  applyDomainBase64Rules(context);
  applyProofEvidenceRules(context);
  applyTestStructureRules(context);
  applyBoundaryTransportRules(context);
}
