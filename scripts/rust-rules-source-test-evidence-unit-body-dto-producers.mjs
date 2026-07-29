import { escapeRegExp } from "./rust-rules-path-core.mjs";

/** Returns whether an expression produces the requested DTO value. */
export function expressionProducesDto(expression, dtoName, factories, producers) {
  const target = escapeRegExp(dtoName);
  if (new RegExp(`\\b${target}\\s*::\\s*from\\s*\\(`, "u").test(expression)) return true;
  if (factories.some((factory) =>
    factory.targetName === dtoName
    && new RegExp(
      `\\b${target}\\s*::\\s*${escapeRegExp(factory.method)}\\s*\\(`,
      "u",
    ).test(expression))) return true;
  return producers.some((producer) =>
    producer.targetName === dtoName
    && new RegExp(`(?:^|[^A-Za-z0-9_])${escapeRegExp(producer.name)}\\s*\\(`, "u").test(expression));
}
