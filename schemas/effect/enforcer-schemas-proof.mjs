import {
  Schema,
  StringArray,
  OptionalStringArray,
  OptionalBoolean,
  OptionalNumber,
  OptionalNullableNumber,
} from "./enforcer-schemas-core.mjs";
import { LanguageSchema, SeveritySchema } from "./enforcer-schemas-rules.mjs";

export const ProofCapabilitySchema = Schema.Literal(
  "ci",
  "local",
  "windows",
  "linux",
  "macos",
  "wsl",
  "android-emulator",
  "android-device",
  "ios-simulator",
  "ios-device",
  "browser",
  "network",
  "cloud",
  "manual-required",
);

export const ProofStatusSchema = Schema.Literal(
  "passed",
  "failed",
  "manual-required",
  "unavailable",
  "waived",
);

export const ProofCollectorSchema = Schema.Literal(
  "command",
  "file-hash",
  "junit",
  "sarif",
  "playwright",
  "cargo",
  "python",
  "android",
  "xcode",
  "http",
  "manual-artifact",
);

export const ProofFamilySchema = Schema.Literal(
  "command",
  "test-report",
  "security-report",
  "contract-parity",
  "manual-artifact",
  "device-manual",
  "event-network",
  "logging-custody",
  "release-package",
  "claim-integrity",
);

export const ProofRetentionPolicySchema = Schema.Struct({
  maxRunsPerProof: OptionalNumber,
  maxFailedRuns: OptionalNumber,
  maxArtifactBytes: OptionalNumber,
  pruneAfterDays: OptionalNullableNumber,
  pinPrReadyDays: OptionalNullableNumber,
});

export const ProofDefinitionSchema = Schema.Struct({
  id: Schema.String,
  title: Schema.String,
  family: ProofFamilySchema,
  severity: SeveritySchema,
  appliesTo: StringArray,
  triggers: StringArray,
  languages: Schema.optional(Schema.Array(LanguageSchema)),
  capabilities: Schema.Array(ProofCapabilitySchema),
  collector: ProofCollectorSchema,
  docs: StringArray,
  commands: Schema.optional(Schema.Array(Schema.Array(Schema.String))),
  requiredArtifacts: OptionalStringArray,
  requiredPaths: OptionalStringArray,
  claimsProved: OptionalStringArray,
  claimsNotProved: OptionalStringArray,
  requiredForPrReady: OptionalBoolean,
  ciSupport: OptionalBoolean,
  deviceSupport: OptionalBoolean,
  retention: Schema.optional(ProofRetentionPolicySchema),
});

export const ProofRegistrySchema = Schema.Struct({
  schemaVersion: Schema.Number,
  productName: Schema.String,
  proofs: Schema.Array(ProofDefinitionSchema),
});
