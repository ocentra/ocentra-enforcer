import type { UiFindingRowResponse } from "../bindings/UiFindingRowResponse";
import type { UiReportResponse } from "../bindings/UiReportResponse";

type ReportFinding = UiFindingRowResponse & {
  doc?: string;
  waiverId?: string;
  waiverOwner?: string;
  waiverReason?: string;
  waiverExpires?: string;
  waiverSource?: string;
};

export type EnforcerReport = UiReportResponse & {
  violations: ReportFinding[];
  warnings: ReportFinding[];
  waived: ReportFinding[];
  runtime?: string;
  persistence?: string;
  generatedAt?: string;
  runId?: string;
  targetLabel?: string;
};

export type DisplayFinding = ReportFinding & {
  id: string;
  category: string;
  status: "open" | "warning" | "waived";
  owner: string;
  summary: string;
  doc?: string;
};

export const emptyReport: EnforcerReport = {
  ok: true,
  scope: "workspace",
  violations: [],
  warnings: [],
  waived: [],
  totalCount: 0,
};

export function displayFindings(report: EnforcerReport, projectOwner: string): DisplayFinding[] {
  const from = (items: ReportFinding[], status: DisplayFinding["status"]): DisplayFinding[] => items.map((finding, index) => ({
    ...finding,
    id: `${finding.ruleId}:${finding.file}:${finding.line}:${index}`,
    category: categoryForRule(finding.ruleId),
    status,
    owner: ownerForFile(finding.file, projectOwner),
    summary: finding.detail,
  }));
  return [
    ...from(report.violations, "open"),
    ...from(report.warnings, "warning"),
    ...from(report.waived, "waived"),
  ];
}

function categoryForRule(ruleId: string) {
  if (ruleId.startsWith("ARCH")) return "Architecture";
  if (ruleId.startsWith("TS")) return "TypeScript source";
  if (ruleId.startsWith("RR-12")) return "Test evidence";
  if (ruleId.startsWith("RR-6")) return "Rust domain";
  if (ruleId.startsWith("RR-7")) return "Imports and modules";
  if (ruleId.startsWith("DOC")) return "Documentation";
  if (ruleId.startsWith("LIT")) return "Literal policy";
  return "Enforcer policy";
}

function ownerForFile(file: string, projectOwner: string) {
  if (file.startsWith("apps/portal")) return "portal";
  if (file.startsWith("packages/asset-editor")) return "asset-editor";
  if (file.includes("frontend")) return "enforcer-ui";
  return projectOwner;
}
