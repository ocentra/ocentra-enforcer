import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import { cargoFmtArgs } from "../scripts/rust-rules-cargo-gates.mjs";
import {
  cleanupTargetArtifacts,
  workspaceTestBatches,
  workspaceTestPlan,
} from "../scripts/check-cargo-workspace-tests.mjs";
import { removeTargetArtifact } from "../scripts/check-cargo-workspace-test-plan.mjs";
import {
  cargoFmtCommand,
  cargoTestCommand,
} from "../scripts/rust-rules-cargo-gates-build.mjs";

test("crate cargo fmt gate stays inside the selected package", () => {
  assert.deepEqual(
    cargoFmtArgs({ mode: "crate", crateName: "enforcer-security" }),
    ["fmt", "--package", "enforcer-security", "--check"],
  );
});

test("workspace cargo fmt gate is explicit about all packages", () => {
  assert.deepEqual(cargoFmtArgs({ mode: "workspace" }), [
    "fmt",
    "--all",
    "--check",
  ]);
});

test("workspace cargo fmt gate uses bounded package batches when the helper exists", () => {
  const command = cargoFmtCommand(process.cwd(), ["fmt", "--all", "--check"]);
  assert.equal(command.command, process.execPath);
  assert.deepEqual(command.args.slice(-1), ["--fmt-check"]);
});

test("workspace cargo tests use the bounded target runner", () => {
  const command = cargoTestCommand(
    process.cwd(),
    ["--workspace"],
    [
      "test",
      "--locked",
      "--workspace",
      "--all-features",
      "--",
      "--test-threads=2",
    ],
  );

  assert.equal(command.command, process.execPath);
  assert.match(command.args[0], /check-cargo-workspace-tests\.mjs$/u);
  assert.deepEqual(command.args.slice(1), ["--test-threads=2"]);
});

test("bounded cargo test plans include every supported target kind", () => {
  const plan = workspaceTestPlan([
    {
      name: "sample",
      targets: [
        { name: "sample", kind: ["lib"] },
        { name: "sample-bin", kind: ["bin"] },
        { name: "sample-tests", kind: ["test"] },
        { name: "sample-example", kind: ["example"] },
        { name: "sample-bench", kind: ["bench"] },
        { name: "build-script", kind: ["custom-build"] },
      ],
    },
  ]);

  assert.deepEqual(plan.map((entry) => entry.selector).sort(), [
    "--bench sample-bench",
    "--bin sample-bin",
    "--example sample-example",
    "--lib",
    "--test sample-tests",
  ]);
});

test("bounded cargo test batches preserve every target once and package boundaries", () => {
  const plan = [
    ...Array.from({ length: 10 }, (_, index) => ({
      packageName: "alpha",
      targetName: `alpha-${index}`,
      kind: "test",
      selectorArgs: ["--test", `alpha-${index}`],
      selector: `--test alpha-${index}`,
    })),
    {
      packageName: "beta",
      targetName: "beta-lib",
      kind: "lib",
      selectorArgs: ["--lib"],
      selector: "--lib",
    },
  ];

  const batches = workspaceTestBatches(plan, 8);

  assert.deepEqual(
    batches.map((batch) => ({
      packageName: batch.packageName,
      targetCount: batch.entries.length,
    })),
    [
      { packageName: "alpha", targetCount: 8 },
      { packageName: "alpha", targetCount: 2 },
      { packageName: "beta", targetCount: 1 },
    ],
  );
  assert.deepEqual(
    batches.flatMap((batch) => batch.entries),
    plan,
  );
  assert.deepEqual(batches[0].selectorArgs.slice(0, 4), [
    "--test",
    "alpha-0",
    "--test",
    "alpha-1",
  ]);
});

test("bounded cargo test batches reject invalid limits", () => {
  assert.throws(
    () => workspaceTestBatches([], 0),
    /batch size must be a positive integer/u,
  );
});

test("bounded target cleanup removes only generated target artifacts", () => {
  const targetDirectory = fs.mkdtempSync(
    path.join(os.tmpdir(), "cargo-bounded-test-"),
  );
  const depsDirectory = path.join(targetDirectory, "debug", "deps");
  fs.mkdirSync(depsDirectory, { recursive: true });
  fs.writeFileSync(path.join(depsDirectory, "sample_tests-abc"), "generated");
  fs.writeFileSync(
    path.join(depsDirectory, "sample_tests-abc.pdb"),
    "generated",
  );
  fs.writeFileSync(
    path.join(depsDirectory, "sample_tests-abc.rlib"),
    "library",
  );
  fs.writeFileSync(path.join(depsDirectory, "other_tests-abc"), "other target");

  try {
    cleanupTargetArtifacts(targetDirectory, {
      targetName: "sample-tests",
      kind: "test",
    });
    assert.equal(
      fs.existsSync(path.join(depsDirectory, "sample_tests-abc")),
      false,
    );
    assert.equal(
      fs.existsSync(path.join(depsDirectory, "sample_tests-abc.pdb")),
      false,
    );
    assert.equal(
      fs.existsSync(path.join(depsDirectory, "sample_tests-abc.rlib")),
      true,
    );
    assert.equal(
      fs.existsSync(path.join(depsDirectory, "other_tests-abc")),
      true,
    );
  } finally {
    fs.rmSync(targetDirectory, { recursive: true, force: true });
  }
});

test("bounded target cleanup retries transient Windows executable locks", () => {
  const calls = [];
  removeTargetArtifact(
    "target/debug/deps/sample_tests-abc.exe",
    (artifactPath, options) => {
      calls.push({ artifactPath, options });
    },
  );

  assert.deepEqual(calls, [
    {
      artifactPath: "target/debug/deps/sample_tests-abc.exe",
      options: {
        force: true,
        recursive: true,
        maxRetries: 6,
        retryDelay: 250,
      },
    },
  ]);
});

test("cargo fmt falls back to the direct invocation outside this repository", () => {
  const command = cargoFmtCommand("C:\\missing-enforcer-project", [
    "fmt",
    "--all",
    "--check",
  ]);
  assert.equal(command.command, "cargo");
  assert.deepEqual(command.args, ["fmt", "--all", "--check"]);
});
