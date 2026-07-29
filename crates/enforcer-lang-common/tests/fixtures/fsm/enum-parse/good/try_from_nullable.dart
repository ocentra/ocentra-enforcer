// PASS fixture for FSM-ENUMPARSE.1: enum parse returns nullable / throws,
// no silent orElse/?? default fallback.

Status? tryParseStatus(String raw) {
  for (final s in Status.values) {
    if (s.name == raw) {
      return s;
    }
  }
  return null;
}
