import { Schema } from "effect";

export const WidgetId = Schema.String.pipe(Schema.brand("WidgetId"));
export type WidgetId = Schema.Schema.Type<typeof WidgetId>;
