#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));

function main() {
  const args = new Set(process.argv.slice(2));
  const checks = [];
  if (args.size === 0 || args.has('--skill')) checks.push(...validateSkills());
  if (args.size === 0 || args.has('--plugin')) checks.push(...validatePlugin());

  const failed = checks.filter((check) => !check.ok);
  const report = {
    ok: failed.length === 0,
    command: 'validate-codex-assets',
    checks,
  };
  console.log(JSON.stringify(report, null, 2));
  if (failed.length > 0) process.exitCode = 1;
}

function validateSkills() {
  const canonicalPath = path.join(PACK_ROOT, 'skills', 'ocentra-enforcer', 'SKILL.md');
  const legacyPath = path.join(PACK_ROOT, 'skills', 'rust-rules-hard-gate', 'SKILL.md');
  const checks = [];
  checks.push(fileCheck('canonical skill exists', canonicalPath));
  checks.push(fileCheck('legacy skill alias exists', legacyPath));
  if (fs.existsSync(canonicalPath)) {
    const parsed = parseSkill(canonicalPath);
    checks.push(valueCheck('canonical skill name', parsed.frontmatter.name === 'ocentra-enforcer', parsed.frontmatter.name));
    checks.push(valueCheck('canonical skill mentions rule index', parsed.body.includes('rules/INDEX.md'), 'rules/INDEX.md'));
    checks.push(valueCheck('canonical skill mentions MCP route', parsed.body.includes('ocentra_enforcer_route'), 'ocentra_enforcer_route'));
    checks.push(valueCheck('canonical skill mentions harness diagnostics', parsed.body.includes('ocentra_enforcer_last_failure'), 'ocentra_enforcer_last_failure'));
    checks.push(valueCheck('canonical skill mentions hard failures', /hard failures|hard-fail|fail hard/iu.test(parsed.body), 'hard failure policy'));
  }
  if (fs.existsSync(legacyPath)) {
    const parsed = parseSkill(legacyPath);
    checks.push(valueCheck('legacy skill name', parsed.frontmatter.name === 'rust-rules-hard-gate', parsed.frontmatter.name));
    checks.push(valueCheck('legacy skill points to Enforcer', parsed.body.includes('ocentra-enforcer'), 'ocentra-enforcer'));
  }
  return checks;
}

function validatePlugin() {
  const pluginPath = path.join(PACK_ROOT, '.codex-plugin', 'plugin.json');
  const mcpPath = path.join(PACK_ROOT, '.mcp.json');
  const checks = [fileCheck('plugin manifest exists', pluginPath), fileCheck('plugin mcp manifest exists', mcpPath)];
  if (!fs.existsSync(pluginPath)) return checks;

  const plugin = JSON.parse(fs.readFileSync(pluginPath, 'utf8'));
  checks.push(valueCheck('plugin name', plugin.name === 'ocentra-enforcer', plugin.name));
  checks.push(valueCheck('plugin skills path', plugin.skills === './skills/', plugin.skills));
  checks.push(valueCheck('plugin mcp path', plugin.mcpServers === './.mcp.json', plugin.mcpServers));
  checks.push(valueCheck('plugin display name', plugin.interface?.displayName === 'Ocentra Enforcer', plugin.interface?.displayName));
  checks.push(valueCheck('plugin prompts mention validation', (plugin.interface?.defaultPrompt ?? []).some((entry) => /validate|enforcer/iu.test(entry)), 'defaultPrompt'));
  checks.push(valueCheck('plugin brand color is set', typeof plugin.interface?.brandColor === 'string' && plugin.interface.brandColor.length > 0, plugin.interface?.brandColor));
  return checks;
}

function parseSkill(filePath) {
  const text = fs.readFileSync(filePath, 'utf8');
  const match = /^---\r?\n(?<frontmatter>[\s\S]*?)\r?\n---\r?\n(?<body>[\s\S]*)$/u.exec(text);
  if (match === null) throw new Error(`${filePath} is missing YAML frontmatter`);
  const frontmatter = {};
  for (const line of match.groups.frontmatter.split(/\r?\n/u)) {
    const separator = line.indexOf(':');
    if (separator <= 0) continue;
    frontmatter[line.slice(0, separator).trim()] = line.slice(separator + 1).trim();
  }
  return { frontmatter, body: match.groups.body };
}

function fileCheck(name, filePath) {
  return {
    name,
    ok: fs.existsSync(filePath),
    detail: normalize(filePath),
  };
}

function valueCheck(name, ok, detail) {
  return {
    name,
    ok: Boolean(ok),
    detail: String(detail ?? ''),
  };
}

function normalize(filePath) {
  return path.resolve(filePath).replaceAll('\\', '/');
}

main();
