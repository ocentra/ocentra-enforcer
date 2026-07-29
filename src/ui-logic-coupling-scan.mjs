/*
 * UI/business-logic coupling scanner: finds presentation-layer files (pages,
 * components, views, screens) that call a business-logic/API module directly
 * instead of going through a dedicated hook/composable — the "button press
 * should not know about business logic" boundary. Signal-based (imports,
 * naming conventions), not an AST parser — this is a mechanical first pass;
 * every finding is evidence to review, not a certified defect.
 *
 * Reusable across any target repo: `scanUiLogicCoupling({ root })`.
 */
import path from "node:path";
import { walk, readTextSafe, relFiles } from "./test-doctrine-fs.mjs";
import {
  isPresentationFile,
  scanUiPresentationFile,
} from "./ui-logic-coupling-helpers.mjs";

const DEFAULT_PRESENTATION_DIR_SEGMENTS = ["pages", "components", "views", "screens", "containers"];
const DEFAULT_BUSINESS_LOGIC_IMPORT_PATTERNS = [
  /\/lib\/api(["'/]|$)/i,
  /\/services\//i,
  /\/api\/client/i,
  /\bapi-client\b/i,
];
const DEFAULT_EVENT_SOURCE_IMPORT_PATTERNS = [/\/lib\/ws(["'/]|$)/i, /\/lib\/socket/i, /\/realtime/i];
function scanUiLogicCoupling({
  root,
  presentationDirSegments = DEFAULT_PRESENTATION_DIR_SEGMENTS,
  businessLogicImportPatterns = DEFAULT_BUSINESS_LOGIC_IMPORT_PATTERNS,
  eventSourceImportPatterns = DEFAULT_EVENT_SOURCE_IMPORT_PATTERNS,
}) {
  const resolvedRoot = path.resolve(root);
  const files = walk(resolvedRoot);
  const relPaths = relFiles(files, resolvedRoot);
  const findings = [];
  for (let i = 0; i < files.length; i += 1) {
    if (!isPresentationFile(relPaths[i], presentationDirSegments)) continue;
    const text = readTextSafe(files[i]);
    findings.push(...scanUiPresentationFile(relPaths[i], text, businessLogicImportPatterns, eventSourceImportPatterns));
  }
  const hard = findings.filter((f) => f.severity === "hard");
  const info = findings.filter((f) => f.severity === "info");
  const filesWithHardFindings = new Set(hard.map((f) => f.file));
  return {
    root: resolvedRoot,
    rule: {
      id: "ARCH-1.16",
      title: "Presentation/UI cannot call business logic directly",
      doc: "rules/common/architecture.md#covered-rules",
      aka: "Humble Object pattern / UI half of Hexagonal (Ports-and-Adapters) architecture / the boundary unidirectional-data-flow (Flux/Redux/Elm) architectures enforce",
      why: "Lets a UI shell be replaced (web/mobile/desktop) without touching business logic, lets business logic be tested without rendering anything, and gives the boundary something to contract-test instead of testing everything through the UI.",
    },
    caveat:
      "Mechanical, signal-based (import paths + naming conventions) — not an AST parser. "
      + "Every finding is evidence for human/AI review, not a certified defect. Run a second pass "
      + "before treating any 'hard' finding as confirmed.",
    findings,
    summary: {
      totalFindings: findings.length,
      hardFindings: hard.length,
      infoFindings: info.length,
      filesWithHardFindings: filesWithHardFindings.size,
    },
    hard,
    info,
  };
}

export { scanUiLogicCoupling };
