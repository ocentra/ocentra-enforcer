import {
  Schema,
  StringArray,
  OptionalBoolean,
} from "./enforcer-schemas-core.mjs";

export const LanguageSchema = Schema.Literal(
  "rust",
  "typescript",
  "python",
  "common",
  "iac",
);

export const RustRuleFamilySchema = Schema.Literal(
  "source",
  "domain",
  "imports-modules",
  "toolchain-cargo",
  "dependencies",
  "async-runtime",
);

export const TypeScriptRuleFamilySchema = Schema.Literal(
  "source",
  "tests",
  "toolchain",
);

export const PythonRuleFamilySchema = Schema.Literal(
  "source",
  "tests",
  "toolchain",
);

export const IacRuleFamilySchema = Schema.Literal(
  "infra-security",
  "infra-toolchain",
);

export const CommonRuleFamilySchema = Schema.Literal(
  "source",
  "security",
  "generated-artifacts",
  "harness",
  "mcp",
  "proof",
  "registry",
  "scanner",
  "documentation",
  "tests",
  "portability",
  "source-shape",
  "contracts",
  "dependencies",
  "sbom",
  "agent-rules",
  "ci",
  "repo",
  "package",
);

export const RuleFamilySchema = Schema.Union(
  RustRuleFamilySchema,
  TypeScriptRuleFamilySchema,
  PythonRuleFamilySchema,
  CommonRuleFamilySchema,
  IacRuleFamilySchema,
);

export const SeveritySchema = Schema.Literal("error", "warning", "info");

export const RuleLockLevelSchema = Schema.Literal(
  "immutable",
  "waiver-required",
  "profile-overridable",
  "advisory",
);

export const RuleEntrySchema = Schema.Struct({
  id: Schema.String,
  language: LanguageSchema,
  family: RuleFamilySchema,
  severity: SeveritySchema,
  title: Schema.String,
  snippet: Schema.String,
  lockLevel: RuleLockLevelSchema,
  canDisable: Schema.Boolean,
  canDowngrade: Schema.Boolean,
  requiresFailFixture: Schema.Boolean,
  requiresPassFixture: Schema.Boolean,
  waivable: OptionalBoolean,
  appliesTo: StringArray,
  triggers: StringArray,
  validator: Schema.String,
  doc: Schema.String,
});

export const RuleRegistrySchema = Schema.Struct({
  schemaVersion: Schema.Number,
  productName: Schema.String,
  languages: Schema.Array(LanguageSchema),
  rules: Schema.Array(RuleEntrySchema),
});
