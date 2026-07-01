# Rust Domain Boundary Rules

Use this doc for `*.rs` files that define domain types, protocol shapes,
serialized envelopes, command/event structures, or public APIs.

## Covered Rules

- `RR-6.1`: no raw string/path types in non-boundary function signatures.
- `RR-6.2`: no primitive types in non-boundary function signatures.
- `RR-6.3`: no public raw fields.
- `RR-6.4`: raw private fields require `BRAND-INVARIANT:`.
- `RR-6.5`: type aliases must not disguise raw primitives.
- `RR-6.6`: tuple newtypes must not expose raw inner fields.
- `RR-6.27`: domain APIs must not accept `AsRef<str>`.
- `RR-6.28`: domain APIs must not accept `Into<String>`.
- `RR-6.29`: ID-like APIs must not accept `impl Display`.
- `RR-6.30`: domain APIs must not expose `Cow<str>`.
- `RR-6.31`: domain APIs must not expose `Vec<String>`.
- `RR-6.32`: domain APIs must not expose `HashMap<String, _>`.
- `RR-6.33`: domain APIs must not expose `BTreeMap<String, _>`.
- `RR-6.34`: domain code must not expose `serde_json::Value`.
- `RR-6.35`: domain structs must not use `Option<String>`.
- `RR-6.36`: domain state must not use `Option<bool>`.
- `RR-6.37`: domain state must not be represented as clusters of booleans.
- `RR-6.38`: named timing values must not use raw `Duration`.
- `RR-6.39`: public domain APIs must not expose raw `SystemTime` or `Instant`.
- `RR-6.40`: URL-like fields and params must not use raw strings.
- `RR-6.41`: path-like fields and params must not use raw strings.
- `RR-6.42`: ID-like fields must not use raw strings or primitives.
- `RR-6.43`: public newtype fields are forbidden; tuple newtype internals stay private.
- `RR-6.45`: newtype constructors must not panic or unwrap validation.
- `RR-6.46`: numeric ID newtypes should use `NonZero*` or validated representations.
- `RR-6.47`: domain collection aliases must be newtypes, not type aliases.
- `RR-6.48`: public/domain signatures must not use naked tuples.
- `RR-6.49`: constructors must not take multiple raw primitive parameters.
- `RR-6.51`: secret-like types must not derive unredacted `Debug`.
- `RR-6.26`: serialized domain fields must not expose raw identity primitives when enabled.

## Agent Rule

Use branded newtypes, enums, and validated constructors. Boundary exceptions
belong in profile globs, not inline bypasses.

## Fails

```rust
pub fn load_user<T: AsRef<str>>(id: T) -> Result<User, LookupError> {
    todo!()
}

pub struct ActivityState {
    active: bool,
    pending: bool,
    failed: bool,
}
```

## Passes

```rust
pub enum ActivityState {
    Active,
    Pending,
    Failed,
}

pub fn load_user(id: UserId) -> Result<User, LookupError> {
    lookup(id)
}
```

## Fix Recipe

1. Decode raw text at adapters or DTO boundaries.
2. Pass branded values, typed collections, and typed map keys into domain code.
3. Replace boolean state clusters with enums or typestate objects.
4. Keep raw serialized fields behind configured generated/DTO owner modules.

## Validator

- scanner: `rust/domain-types`
- command: `ocentra-enforcer scan --root <repo> --files <file.rs>`
