export function normalizeWaiverToday(value) {
  if (value == null) return new Date().toISOString().slice(0, 10);
  return normalizeWaiverDate(value, "today");
}

export function normalizeWaiverDate(value, label) {
  const date = String(value ?? "");
  if (!/^\d{4}-\d{2}-\d{2}$/u.test(date) || !isRealCalendarDay(date)) {
    throw new Error(`${label} must be a real YYYY-MM-DD date.`);
  }
  return date;
}

function isRealCalendarDay(date) {
  const parsed = new Date(`${date}T00:00:00.000Z`);
  return !Number.isNaN(parsed.getTime())
    && parsed.getUTCFullYear() === Number(date.slice(0, 4))
    && parsed.getUTCMonth() + 1 === Number(date.slice(5, 7))
    && parsed.getUTCDate() === Number(date.slice(8, 10));
}
