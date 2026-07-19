import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import { escapeRegExp } from "./rust-rules-path-core.mjs";
import {
  roundTripFactoryDescriptors,
  roundTripValueProducerDescriptors,
} from "./rust-rules-source-test-evidence-roundtrip-associated-descriptors.mjs";
import {
  executableBody,
  rustTestBodies,
} from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import { hasAssociatedConversionRejection } from "./rust-rules-source-test-evidence-unit-body-conversion-proof.mjs";

function persistedDtoFields(source, dtoName) {
  const declaration = new RegExp(
    `\\bstruct\\s+${escapeRegExp(dtoName)}\\s*\\{(?<fields>[\\s\\S]*?)\\n\\s*\\}`,
    "u",
  );
  const fields = declaration.exec(source)?.groups?.fields ?? "";
  return [...fields.matchAll(/^\s*(?<name>[a-z][A-Za-z0-9_]*)\s*:/gmu)]
    .map((match) => match.groups?.name ?? "")
    .filter(Boolean);
}

function hasPersistedDtoRejection(body, source, dtoName) {
  const fields = persistedDtoFields(source, dtoName);
  const wireName = (field) => field.replace(/_([a-z])/gu, (_, letter) => letter.toUpperCase());
  return fields.length > 0
    && fields.every((field) => {
      const names = [field, wireName(field)].map(escapeRegExp).join("|");
      return new RegExp(`['\"](?:${names})['\"]\\s*:`, "u").test(body);
    })
    && /\b(?:expect_err|assert(?:_matches)?!\s*\([^)]*\.is_err\(\)|match\s+[^{}]+\bErr\b)/u.test(body);
}

/** Checks that a fallible DTO conversion has an associated rejection assertion. */
export function hasConversionRejectionEvidence(
  root,
  filePath,
  source,
  dtoName,
  domainName,
  evidenceContext = null,
) {
  const evidencePattern = /(?:^|[^A-Za-z0-9])(?:invalid|reject[A-Za-z0-9_]*|malformed|bad_input|unparseable)(?:$|[^A-Za-z0-9])/iu;
  const sources = evidenceContext?.sources ?? crateEvidenceSources(root, filePath, source);
  const testBodies = evidenceContext?.rustTestBodies ?? sources.map(rustTestBodies);
  const rawTestBodies = sources.map((evidenceSource) => rustTestBodies(evidenceSource, evidenceSource));
  const factories = evidenceContext?.roundTripFactories
    ?? sources.flatMap(roundTripFactoryDescriptors);
  const producers = evidenceContext?.roundTripValueProducers
    ?? sources.flatMap(roundTripValueProducerDescriptors);
  return testBodies.some((bodies, sourceIndex) =>
    bodies.some((testBody, bodyIndex) => {
      const body = executableBody(testBody);
      const rawTestBody = rawTestBodies[sourceIndex]?.[bodyIndex] ?? "";
      if (evidencePattern.test(rawTestBody)
        && hasPersistedDtoRejection(executableBody(rawTestBody), source, dtoName)) return true;
      return evidencePattern.test(testBody)
        && hasAssociatedConversionRejection(
          body,
          dtoName,
          domainName,
          factories,
          producers,
        );
    }));
}
