export function manifestJsonParseFailure(packageJsonPath, error) {
  return {
    ok: false,
    findings: [
      {
        ruleId: "NPM-1.3",
        severity: "error",
        title: "package.json is not valid JSON",
        detail: `package.json is not valid JSON: ${error instanceof Error ? error.message : String(error)}`,
        file: packageJsonPath,
        line: 1,
        snippet: null,
        source: null,
        doc: "rules/common/dependencies.md#covered-rules",
      },
    ],
  };
}
