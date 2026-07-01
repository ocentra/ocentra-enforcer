# Rust Source Rules

Use this doc for `*.rs` files when routing general source safety,
error-handling, allocation, panic, unsafe, lint-suppression, and runtime string
rules.

## Covered Rules

- `RR-2.1`: no `#[allow(...)]` or `#[expect(...)]` lint suppressions.
- `RR-2.2`: no validator suppression comments.
- `RR-3.1`: unsafe Rust is forbidden by default.
- `RR-3.2`: unsafe blocks require nearby `SAFETY:` documentation when unsafe is enabled.
- `RR-3.3`: unsafe functions require a `# Safety` section when unsafe is enabled.
- `RR-3.4`: raw pointers are forbidden in public/domain APIs.
- `RR-3.16`: `transmute` is forbidden.
- `RR-3.17`: `MaybeUninit` is forbidden outside approved unsafe owners.
- `RR-3.18`: `ManuallyDrop` requires unsafe justification.
- `RR-3.19`: `mem::forget` is forbidden.
- `RR-3.20`: `Box::leak` requires `LEAK-JUSTIFICATION:`.
- `RR-3.21`: `static mut` is forbidden.
- `RR-3.22`: `UnsafeCell` is forbidden outside approved primitives.
- `RR-3.23`: `unsafe impl Send/Sync` requires `SAFETY:` proof.
- `RR-3.24`: `get_unchecked` is forbidden.
- `RR-3.25`: raw pointer dereference requires local `SAFETY:` proof.
- `RR-3.26`: FFI extern blocks must live in FFI owner modules.
- `RR-3.27`: FFI structs require `#[repr(C)]`.
- `RR-3.28`: `#[no_mangle]` is restricted to FFI owner modules.
- `RR-3.32`: unsafe code is forbidden in tests as an escape hatch.
- `RR-3.33`: `allow(unsafe_code)` is forbidden.
- `RR-4.1`: no `.unwrap()` or `.expect()`.
- `RR-4.2`: no panic-like production macros.
- `RR-4.3`: no `dbg!`, `println!`, or `eprintln!` in Rust logic.
- `RR-4.4`: no erased application errors in non-boundary domain code.
- `RR-4.7`: no `Result<T, String>` in domain code.
- `RR-4.8`: no `Result<T, &'static str>` in domain code.
- `RR-4.9`: no `Err("literal")` string errors.
- `RR-4.10`: no `Err(format!(...))` string errors.
- `RR-4.11`: no `map_err(|e| e.to_string())` error erasure.
- `RR-4.12`: boolean success APIs are forbidden for fallible operations.
- `RR-4.13`: sentinel error values are forbidden.
- `RR-4.14`: fallible constructors must return `Result<Self, Error>`.
- `RR-4.15`: `main` must not swallow fallible errors.
- `RR-4.16`: no ignored fallible-looking results with `let _ =`.
- `RR-4.17`: no `.ok()` swallowing of `Result` errors.
- `RR-4.18`: no `.unwrap_or_default()` on fallible domain/config data.
- `RR-4.19`: no `.unwrap_or(...)` hiding parse/config failures.
- `RR-4.20`: custom error enums must derive/implement `Debug` and `Error`.
- `RR-4.21`: wrapped source errors require `#[source]` or `#[from]`.
- `RR-4.22`: do not log and return the same error in one function.
- `RR-5.1`: clone requires `CLONE-JUSTIFICATION:`.
- `RR-5.2`: string allocation requires `ALLOC-JUSTIFICATION:`.
- `RR-5.3`: unchecked indexing and slicing are forbidden.
- `RR-5.4`: lossy casts require `CAST-JUSTIFICATION:`.
- `RR-12.22`: no weak `assert!(result.is_ok())`.
- `RR-12.23`: no weak `assert!(option.is_some())`.
- `RR-14.16`: non-boundary domain structs must not derive `Deserialize` directly.
- `RR-14.18`: `#[serde(untagged)]` requires `SERDE-UNTAGGED-JUSTIFICATION:`.
- `RR-18.16`: runtime Rust source must not contain inline string literals when enabled.

## Agent Rule

Fix code structure. Do not add lint disables, suppression comments, or config
downgrades to pass this family.

## Fails

```rust
pub fn parse_user(raw: &str) -> Result<UserId, String> {
    raw.parse().map_err(|e| e.to_string()).ok().unwrap_or_default()
}
```

```rust
static mut CACHE_READY: bool = false;
let id = unsafe { core::mem::transmute::<u64, UserId>(raw) };
```

```rust
#[derive(Deserialize)]
#[serde(untagged)]
pub enum UserEnvelope {
    Named { name: String },
}
```

## Passes

```rust
pub enum UserParseError {
    Empty,
    InvalidFormat,
}

pub fn parse_user(raw: UserIdText) -> Result<UserId, UserParseError> {
    UserId::try_new(raw)
}
```

## Fix Recipe

1. Preserve typed errors; do not erase them into strings or options.
2. Convert raw data at boundary modules before entering domain APIs.
3. Replace global mutable state with synchronized owners.
4. Deserialize into DTOs at boundaries, then validate into domain types.
5. Replace weak assertions with exact value or exact error assertions.

## Validator

- scanner: `rust/source-scan`
- command: `ocentra-enforcer scan --root <repo> --files <file.rs>`
