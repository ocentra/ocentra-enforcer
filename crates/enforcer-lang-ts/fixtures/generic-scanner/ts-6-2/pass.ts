export function decode(raw: string): Widget {
  return Schema.decodeUnknownSync(WidgetSchema)(raw);
}
