import { Schema } from "effect";

export { Schema };

export const ProductName = "ocentra-enforcer";

export const StringArray = Schema.Array(Schema.String);
export const OptionalStringArray = Schema.optional(StringArray);
export const OptionalBoolean = Schema.optional(Schema.Boolean);
export const OptionalString = Schema.optional(Schema.String);
export const OptionalNumber = Schema.optional(Schema.Number);
export const OptionalNullableNumber = Schema.optional(
  Schema.NullOr(Schema.Number),
);
