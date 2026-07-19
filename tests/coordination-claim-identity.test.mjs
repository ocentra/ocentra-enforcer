import assert from "node:assert/strict";
import test from "node:test";

import { applyReleaseEvent } from "../src/coordination/vendor/materialize-claim-identity.js";

test("multi-path release evaluates each active claim once", () => {
  const count = 128;
  const released = 64;
  const activeClaims = new Map();
  for (let index = 0; index < count; index += 1) {
    activeClaims.set(`claim-${index}`, {
      writer: "node_release_perf.codex-a", lane: "codex-a", paths: [`src/claimed-${index}.ts`], eventId: `claim-${index}`,
      context: { projectId: "release-performance-project", repoRoot: "C:/release-performance", worktreeRoot: "C:/release-performance", branch: "feature/release-performance", codexThreadId: "thread-release-performance", operation: "edit", lockKind: "writeLock" },
    });
  }
  const paths = Array.from({ length: count }, (_, index) => `src/unclaimed-${index}.ts`);
  paths[paths.length - 1] = `src/claimed-${released}.ts`;
  let overlapCallCount = 0;
  applyReleaseEvent(activeClaims, {
    writer: "node_release_perf.codex-a", lane: "codex-a", paths, eventId: "release-event",
    context: { projectId: "release-performance-project", repoRoot: "C:/release-performance", worktreeRoot: "C:/release-performance", branch: "feature/release-performance", codexThreadId: "thread-release-performance", operation: "edit", lockKind: "writeLock", explicitReleaseScope: true, releaseClaimEventIds: [`claim-${released}`] },
  }, (left, right) => { overlapCallCount += 1; return left.filter((entry) => right.includes(entry)); });
  assert.equal(activeClaims.size, count - 1);
  assert.equal(activeClaims.has(`claim-${released}`), false);
  assert.equal(overlapCallCount, count);
});
