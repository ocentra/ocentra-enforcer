import test from 'node:test';
import assert from 'node:assert/strict';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-fixture.mjs';
test('clone without justification fails with RR-5.1', () => {
  const project = makeProject({
    'src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
pub struct UserId(NonZeroU64);
impl Clone for UserId {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}
`,
  });
  expectFailure(project, 'RR-5.1');
});

test('clone with justification passes clone policy', () => {
  const project = makeProject({
    'src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
#[derive(Debug)]
pub struct UserId(NonZeroU64);
impl Clone for UserId {
    fn clone(&self) -> Self {
        // CLONE-JUSTIFICATION: NonZeroU64 is copy-like and no ownership aliasing is introduced.
        Self(self.0.clone())
    }
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('unsafe fails with RR-3.1', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: UserId) -> Option<UserId> {
    unsafe { core::hint::unreachable_unchecked() }
}
`,
  });
  expectFailure(project, 'RR-3.1');
});

test('wildcard import fails with RR-7.1', () => {
  const project = makeProject({
    'src/lib.rs': `
use crate::domain::*;
mod domain { pub struct UserId; }
pub struct UserRecord;
`,
  });
  expectFailure(project, 'RR-7.1');
});

test('pub use outside facade fails with RR-7.3', () => {
  const project = makeProject({
    'src/domain/mod.rs': `
pub use crate::other::UserRecord;
`,
    'src/lib.rs': `
mod other { pub struct UserRecord; }
pub mod domain;
`,
  });
  expectFailure(project, 'RR-7.3');
});

test('pub use fails even in facade when profile forbids public re-exports', () => {
  const project = makeProject({
    'src/lib.rs': `
mod domain { pub struct UserRecord; }
pub use domain::UserRecord;
`,
  });
  expectFailure(project, 'RR-7.3');
});

test('facade-only profile allows public re-export in configured facade file', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      schemaVersion: 2,
      profileName: 'strict',
      publicReexportPolicy: 'facade-only',
    }),
    'src/lib.rs': `
mod domain { pub struct UserRecord; }
pub use domain::UserRecord;
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('lint allow suppression fails with RR-2.1', () => {
  const project = makeProject({
    'src/lib.rs': `
#![allow(dead_code)]
pub struct UserId;
`,
  });
  expectFailure(project, 'RR-2.1');
});
