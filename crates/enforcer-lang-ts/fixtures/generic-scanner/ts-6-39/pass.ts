export function total(items: Item[]): number {
  const sum = items.reduce((acc, item) => acc + item.price, 0);
  return sum;
}
