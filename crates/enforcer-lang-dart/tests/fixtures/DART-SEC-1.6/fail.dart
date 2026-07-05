// FAIL fixture for DART-SEC-1.6: unguarded debug output (scored).
class Diagnostics {
  void log(String message) {
    debugPrint('diag: $message');
  }
}
