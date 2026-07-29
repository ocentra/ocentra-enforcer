// PASS fixture for DART-SEC-1.6: debug output guarded by kDebugMode.
class Diagnostics {
  void log(String message) {
    if (kDebugMode) {
      debugPrint('diag: $message');
    }
  }
}
