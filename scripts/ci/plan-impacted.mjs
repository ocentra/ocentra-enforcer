#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import process from 'node:process';
import { pathToFileURL } from 'node:url';
import { classifyChangedFiles } from './plan-impacted-classification.mjs';
import { impactedPackageNames } from './plan-impacted-dependents.mjs';

/** Resolve directly changed Cargo packages and all reverse workspace dependents. */
export function planImpacted({ changedFiles, metadata }) {
  const { normalized, docsOnly, graphContractChanged } = classifyChangedFiles(changedFiles);
  if (docsOnly) {
    return { docsOnly, fullRequired: false, graphContractChanged, packages: [] };
  }

  const packages = impactedPackageNames(metadata, normalized);
  return {
    docsOnly,
    fullRequired: graphContractChanged || packages.length === 0,
    graphContractChanged,
    packages,
  };
}

function main(argv) {
  const args = parseArgs(argv);
  const changedFiles = gitLines(['diff', '--name-only', args.base, args.head]);
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    windowsHide: true,
  }));
  const plan = planImpacted({ changedFiles, metadata });
  const output = {
    ...plan,
    packagesCsv: plan.packages.join(','),
  };
  if (process.env.GITHUB_OUTPUT) {
    for (const [key, value] of Object.entries({
      docs_only: String(plan.docsOnly),
      full_required: String(plan.fullRequired),
      graph_contract_changed: String(plan.graphContractChanged),
      packages: output.packagesCsv,
    })) {
      appendFileSync(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
    }
  }
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function gitLines(args) {
  return execFileSync('git', args, { encoding: 'utf8', windowsHide: true })
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

function parseArgs(argv) {
  const result = { base: 'HEAD^', head: 'HEAD' };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--base') result.base = argv[++index];
    else if (argv[index] === '--head') result.head = argv[++index];
    else throw new Error(`Unknown argument: ${argv[index]}`);
  }
  return result;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main(process.argv.slice(2));
