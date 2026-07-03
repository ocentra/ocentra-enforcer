const DEFAULT_MAX_FILE_BYTES = 2 * 1024 * 1024;
const DEFAULT_MIN_SCORE = 40;

export function buildLiteralRiskOptions(config = {}, args = {}) {
  const policy = config.literalRisk ?? {};
  return {
    minScore:
      args.minScore ??
      args.literalRiskMinScore ??
      policy.minScore ??
      DEFAULT_MIN_SCORE,
    includeLow:
      args.includeLow ?? args.literalRiskIncludeLow ?? policy.includeLow ?? true,
    includeIgnored:
      args.includeIgnored ??
      args.literalRiskIncludeIgnored ??
      policy.includeIgnored ??
      false,
    includeUnknownCode:
      args.includeUnknownCode ??
      args.literalRiskIncludeUnknownCode ??
      policy.includeUnknownCode ??
      false,
    respectGitignore:
      args.respectGitignore ??
      args.literalRiskRespectGitignore ??
      policy.respectGitignore ??
      true,
    maxFileBytes:
      args.maxFileBytes ??
      args.literalRiskMaxFileBytes ??
      policy.maxFileBytes ??
      DEFAULT_MAX_FILE_BYTES,
    failAbove:
      args.failAbove ??
      args.literalRiskFailAbove ??
      policy.failAbove ??
      null,
    hardCategories: uniqueNormalizedList([
      ...(Array.isArray(args.hardCategories) ? args.hardCategories : []),
      ...(Array.isArray(args.literalRiskHardCategories)
        ? args.literalRiskHardCategories
        : []),
      ...(Array.isArray(policy.hardCategories) ? policy.hardCategories : []),
    ]),
    hardRuleIds: uniqueNormalizedList([
      ...(Array.isArray(args.hardRuleIds) ? args.hardRuleIds : []),
      ...(Array.isArray(args.literalRiskHardRuleIds)
        ? args.literalRiskHardRuleIds
        : []),
      ...(Array.isArray(policy.hardRuleIds) ? policy.hardRuleIds : []),
    ]),
  };
}

function uniqueNormalizedList(values) {
  return [...new Set(values.map((value) => String(value ?? "").trim()).filter(Boolean))];
}
