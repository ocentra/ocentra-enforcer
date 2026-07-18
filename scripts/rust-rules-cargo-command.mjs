import process from "node:process";
import { spawnSync } from "node:child_process";
import { RULES } from "../src/rule-metadata.mjs";
import { policyForTool } from "../src/policy.mjs";

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
      source: output.slice(0, 4000),
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

export { commandExists, runCommand, configuredCargoCommand };
