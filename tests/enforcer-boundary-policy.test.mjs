import assert from "node:assert/strict";
import test from "node:test";
import { makeProject, runGateArgs } from "./rust-rules-fixture.mjs";

function scanConfig(rawTypeBoundaryGlobs) {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
      rawTypeBoundaryGlobs,
      boundaryOwnerNote: "Boundary DTO ownership is reviewed by the domain team.",
    }),
  });
  return runGateArgs(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "ocentra-enforcer.config.json",
  ]);
}

function hasBoundaryScopeFinding(result) {
  return JSON.parse(result.stdout).violations.some(
    (violation) => violation.ruleId === "BOUND-1.7",
  );
}

test("boundary ownership globs reject catch-all scopes and accept named scopes", () => {
  assert.equal(hasBoundaryScopeFinding(scanConfig(["src/**"])), true);
  assert.equal(hasBoundaryScopeFinding(scanConfig(["src/boundary/**"])), false);
});
