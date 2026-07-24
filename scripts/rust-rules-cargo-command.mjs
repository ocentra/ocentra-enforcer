import process from "node:process";
import { spawnSync } from "node:child_process";
import { RULES } from "../src/rule-metadata.mjs";
import { policyForTool } from "../src/policy.mjs";

const MAX_COMMAND_OUTPUT = 4000;

/**
 * Keep both the beginning and end of a failed tool's output. Cargo commonly
 * emits thousands of compile lines before the useful linker/test error, so a
 * head-only slice can turn a real failure into an unactionable RR-10 finding.
 */
function summarizeCommandOutput(output, maxLength = MAX_COMMAND_OUTPUT) {
  if (output.length <= maxLength) return output;
  const marker = "\n... [output truncated; tail preserved] ...\n";
  const available = Math.max(0, maxLength - marker.length);
  const headLength = Math.ceil(available / 2);
  const tailLength = available - headLength;
  return `${output.slice(0, headLength)}${marker}${output.slice(-tailLength)}`;
}

function commandExists(command) {
  const result = spawnSync(command, ["--version"], {
    encoding: "utf8",
    stdio: "pipe",
    shell: false,
  });
  return result.status === 0;
}

function runCommand(root, command, args, ruleId, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: { ...process.env, ...env },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: false,
  });
  if (result.status === 0) return [];
  const output = [result.stdout, result.stderr]
    .filter(Boolean)
    .join("\n")
    .trim();
  return [
    {
      ruleId,
      title: RULES[ruleId].title,
      detail: `${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}.`,
      file: ".",
      line: 1,
      snippet: RULES[ruleId].snippet,
      source: summarizeCommandOutput(output),
    },
  ];
}

function configuredCargoCommand(
  root,
  config,
  toolId,
  defaultEnabled,
  command,
  args,
  ruleId,
  env = {},
) {
  const toolPolicy = policyForTool(toolId, config, {
    enabled: defaultEnabled,
    severity: "error",
  });
  if (!toolPolicy.enabled) return [];
  return runCommand(root, command, args, ruleId, env).map((finding) => ({
    ...finding,
    severity: toolPolicy.severity,
  }));
}

export { commandExists, runCommand, configuredCargoCommand, summarizeCommandOutput };
