const CODEC_PREFIX = String.raw`\b(?:bincode|ciborium|postcard|quick_xml|rmp_serde|serde_json|serde_yaml|toml)(?:::[A-Za-z_][A-Za-z0-9_]*)*::`;

/** Matches supported encoding operations in Rust test source. */
export const ENCODE_OPERATION = new RegExp(
  `${CODEC_PREFIX}(?:encode(?:_to_vec)?|into_writer|serialize|to_(?:(?:string|vec)(?:_pretty)?|value|writer))\\s*(?:::<[^>{}]+>)?\\s*\\(`,
  "u",
);

/** Matches supported decoding operations in Rust test source. */
export const DECODE_OPERATION = new RegExp(
  `${CODEC_PREFIX}(?:decode(?:_from_slice)?|deserialize|from_(?:reader|slice|str|value))\\s*(?:::<[^>{}]+>)?\\s*\\(`,
  "u",
);

/** Matches Rust assertion macros accepted as proof evidence. */
export const ASSERTION = /\b(?:assert(?:_eq|_ne)?|debug_assert(?:_eq|_ne)?|matches)\s*!\s*\(/u;
/** Matches Rust equality assertion macros accepted as proof evidence. */
export const EQUALITY_ASSERTION = /\b(?:assert_eq|debug_assert_eq|prop_assert_eq)\s*!\s*\(/u;

/** Reports whether Rust evidence invokes a supported serialization codec. */
export function usesRoundTripEncode(source) {
  return ENCODE_OPERATION.test(source);
}

/** Reports whether Rust evidence invokes a supported deserialization codec. */
export function usesRoundTripDecode(source) {
  return DECODE_OPERATION.test(source);
}
