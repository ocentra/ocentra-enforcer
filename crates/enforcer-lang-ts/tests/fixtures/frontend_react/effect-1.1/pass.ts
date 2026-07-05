import { Schema } from "@effect/schema";

export const OrderSchema = Schema.Struct({
  id: Schema.String,
  total: Schema.Number,
});
