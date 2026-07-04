export function widget(maybe: Widget | undefined): Widget {
  if (maybe === undefined) {
    throw new Error("widget missing");
  }
  const present = maybe !== null && maybe.clone();
  return present !== false ? present : maybe.clone();
}
