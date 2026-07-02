import { formatProofReport } from "./proof-cli-format.mjs";
import { parseProofCli } from "./proof-cli-parse.mjs";

export function stripNullish(value) {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== null && entry !== undefined),
  );
}

export { formatProofReport, parseProofCli };
