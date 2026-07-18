import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { cargoAuditIgnoredAdvisories } from "../src/check-governance-cargo-audit.mjs";

test("cargo audit ignore entries are read from deny.toml advisories policy", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-dep-policy-"));
  fs.writeFileSync(
    path.join(root, "deny.toml"),
    `[advisories]
unmaintained = "all"
yanked = "deny"
ignore = [
  # D-15 tokenizer policy exception.
  "RUSTSEC-2024-0436",
]

[licenses]
allow = ["MIT"]
`,
  );

  assert.deepEqual(cargoAuditIgnoredAdvisories(root), ["RUSTSEC-2024-0436"]);
});
