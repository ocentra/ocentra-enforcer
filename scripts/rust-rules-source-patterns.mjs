// boundaryOwnerNote: Enforcer-owned Rust scan engine patterns.
const RAW_STRING_TYPE_RE =
  /\b(?:String|str|PathBuf|OsString|CString|CStr)\b|\b(?:std|alloc)::(?:string::String|path::PathBuf|ffi::(?:OsString|CString|CStr))\b|\bCow\s*<[^>]*\bstr\b[^>]*>/u;
const RAW_PRIMITIVE_TYPE_RE =
  /\b(?:bool|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64)\b/u;
const RAW_POINTER_RE = /\*(?:const|mut)\s+[A-Za-z_]/u;
const TYPE_ALIAS_RAW_RE =
  /^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+[A-Z][A-Za-z0-9_]*\s*=\s*([^;]+);/u;
const PUBLIC_SERDE_STRUCT_RE = /^\s*pub\s+struct\s+\w+/u;
const PUBLIC_FIELD_RE =
  /^\s*pub\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,]+),?/u;
const FIELD_RE =
  /^\s*(?:pub(?:\([^)]*\))?\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?<type>[^,]+),?/u;
const ID_LIKE_NAME_RE = /(?:^|_)(?:id|ids|key|ref|refs)$/iu;
const URL_LIKE_NAME_RE = /(?:^|_)(?:url|uri|endpoint)$/iu;
const PATH_LIKE_NAME_RE = /(?:^|_)(?:path|file|dir|directory)$/iu;
const TIME_LIKE_NAME_RE =
  /(?:^|_)(?:timeout|ttl|delay|interval|deadline|duration)$/iu;
const FALLIBLE_FN_NAME_RE =
  /^(?:save|load|parse|decode|find|get|lookup|create|open|connect|send|remove|delete|update|write)/u;

export const RAW_TYPE_PATTERNS = {
  RAW_POINTER_RE,
  RAW_PRIMITIVE_TYPE_RE,
  RAW_STRING_TYPE_RE,
  TYPE_ALIAS_RAW_RE,
};

export const STRUCT_FIELD_PATTERNS = {
  FIELD_RE,
  PUBLIC_FIELD_RE,
  PUBLIC_SERDE_STRUCT_RE,
};

export const NAME_PATTERNS = {
  FALLIBLE_FN_NAME_RE,
  ID_LIKE_NAME_RE,
  PATH_LIKE_NAME_RE,
  TIME_LIKE_NAME_RE,
  URL_LIKE_NAME_RE,
};
