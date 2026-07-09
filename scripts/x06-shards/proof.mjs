import { classifyTarget, countByCategory } from "./discovery.mjs";

export function buildProof(allTargets, selectedTargets, args, result) {
  return {
    schemaVersion: 1,
    artifact: "x06-test-shards",
    generatedAt: new Date().toISOString(),
    package: "enforcer-memory",
    testRoot: "crates/enforcer-memory/tests",
    totalTargets: allTargets.length,
    selectedTargets: selectedTargets.length,
    shard: args.shard ? `${args.shard.index}/${args.shard.total}` : "all",
    only: args.only,
    byCategory: countByCategory(allTargets),
    commandTemplate: "node scripts/x06-enforcer-memory-sharded-test.mjs --shard N/M",
    executionPolicy: executionPolicy(),
    result,
    targets: selectedTargets.map(targetProof),
  };
}

function executionPolicy() {
  return {
    deterministicDiscovery: true,
    crossPlatform: true,
    oneCargoTestTargetPerProcess: true,
    avoidsMonolithicPackageTimeout: true,
    zeroNetwork: true,
  };
}

function targetProof(target) {
  return {
    target,
    category: classifyTarget(target),
    cargoArgs: ["test", "-p", "enforcer-memory", "--test", target, "--quiet", "-j", "1"],
  };
}
