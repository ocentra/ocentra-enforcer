export function withCachedArchitectureReports(deps) {
  const scannerReports = new Map();
  const genericReports = new Map();
  return {
    ...deps,
    runEnforcerScan(options, invocationDeps) {
      const key = JSON.stringify({
        root: options.root,
        rawScope: options.rawScope,
        command: options.command,
        scanOnly: options.scanOnly,
        languages: options.languages,
      });
      if (!scannerReports.has(key)) {
        scannerReports.set(key, deps.runEnforcerScan(options, invocationDeps));
      }
      return scannerReports.get(key);
    },
    runGenericScan(options) {
      const key = JSON.stringify({
        root: options.root,
        scope: options.scope,
      });
      if (!genericReports.has(key)) {
        genericReports.set(
          key,
          deps.runGenericScan({
            ...options,
            languages: ["rust", "typescript", "python", "common"],
            sourceOnly: true,
          }),
        );
      }
      return genericReports.get(key);
    },
  };
}
