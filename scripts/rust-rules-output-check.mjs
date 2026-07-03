const PLURAL_SUFFIXES = ["s", ""];
const SCOPE_LABELS = {
  crate: (scope) => `crate ${scope.crateName}`,
  diff: (scope) => `diff ${scope.base}..${scope.head}`,
  files: () => "explicit files",
  workspace: () => "workspace",
};

function printFindingList(findings, label) {
  for (const finding of findings) {
    console.error(
      `${finding.file}:${finding.line}: ${finding.severity ?? "error"} ${finding.ruleId} ${finding.title}`,
    );
    console.error(`  ${label}: ${finding.detail}`);
    console.error(`  Rule: ${finding.doc ?? finding.docPath ?? ""}`);
    console.error(`  Fix: ${finding.snippet}`);
    if (finding.source) {
      for (const line of String(finding.source).split(/\r?\n/u).slice(0, 12))
        console.error(`  > ${line}`);
    }
    console.error("");
  }
}

function pluralSuffix(count) {
  return PLURAL_SUFFIXES[Number(count === 1)];
}

function describeScope(scope) {
  return (SCOPE_LABELS[scope.mode] ?? SCOPE_LABELS.workspace)(scope);
}

export function printCheckReport(report) {
  const label = report.check ?? report.command ?? "check";
  const warnings = report.warnings ?? [];
  if (report.violations.length === 0 && warnings.length === 0) {
    console.log(`Ocentra Enforcer ${label} passed.`);
    return;
  }
  if (report.violations.length === 0) {
    console.log(
      `Ocentra Enforcer ${label} passed with ${warnings.length} warning${pluralSuffix(warnings.length)}.`,
    );
    printFindingList(warnings, "Warning");
    return;
  }
  console.error(
    `Ocentra Enforcer ${label} failed with ${report.violations.length} violation${pluralSuffix(report.violations.length)}.`,
  );
  console.error(`Profile: ${report.profileName}`);
  console.error("");
  printFindingList(report.violations, "Reason");
  warnings.length > 0 && printFindingList(warnings, "Warning");
}

export function printScanReport(report) {
  const warnings = report.warnings ?? [];
  if (report.violations.length === 0 && warnings.length === 0) {
    console.log(
      `Ocentra Enforcer ${report.command} passed for ${report.scope.files.length} file(s).`,
    );
    return;
  }

  if (report.violations.length === 0) {
    console.log(
      `Ocentra Enforcer ${report.command} passed with ${warnings.length} warning${pluralSuffix(warnings.length)} for ${report.scope.files.length} file(s).`,
    );
    printFindingList(warnings, "Warning");
    return;
  }

  console.error(
    `Ocentra Enforcer ${report.command} failed with ${report.violations.length} violation${pluralSuffix(report.violations.length)}.`,
  );
  console.error(`Profile: ${report.profileName}`);
  console.error(`Scope: ${describeScope(report.scope)}`);
  console.error(
    `Failing severities: ${(report.failOn ?? ["error"]).join(", ")}`,
  );
  console.error("");
  printFindingList(report.violations, "Reason");
  warnings.length > 0 && printFindingList(warnings, "Warning");
}
