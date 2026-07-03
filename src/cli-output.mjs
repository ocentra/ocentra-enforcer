export function emitJson(value) {
  console.log(JSON.stringify(value, null, 2));
}

export function emitText(text) {
  console.log(text);
}

export function emitPrintedReport({ json, report, printer }) {
  if (json) emitJson(report);
  else printer(report);
  return report.ok ? 0 : 1;
}

export function emitAlwaysOk({ json, report, printer }) {
  if (json) emitJson(report);
  else printer(report);
  return 0;
}

export function emitMaybeOk({ json, report, printer }) {
  if (json) emitJson(report);
  else printer(report);
  return report?.ok === false ? 1 : 0;
}
