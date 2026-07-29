export function normalizeWaiverToday(value) {
  if (value == null) {
    const parts = new Intl.DateTimeFormat("en-CA", {
      timeZone: "UTC",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    }).formatToParts();
    const byType = new Map(parts.map((part) => [part.type, part.value]));
    return `${byType.get("year")}-${byType.get("month")}-${byType.get("day")}`;
  }
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
  const [year, month, day] = date.split("-").map(Number);
  if (month < 1 || month > 12 || day < 1) return false;
  const daysInMonth = [
    31,
    year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28,
    31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
  ];
  return day <= daysInMonth[month - 1];
}
