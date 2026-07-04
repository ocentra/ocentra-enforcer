// FAIL fixture for FSM-ENUMPARSE.1: enum parse with a silent orElse
// fallback variant instead of throwing or returning nullable.

Status parseStatus(String raw) {
  return Status.values.firstWhere(
    (s) => s.name == raw,
    orElse: () => Status.pending,
  );
}
