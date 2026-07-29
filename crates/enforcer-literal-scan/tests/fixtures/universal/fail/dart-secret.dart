// Fail fixture for LIT-2.1 (universal literal-scan T2 advisory bridge).
// Dense with hardcoded secret-shaped and literal-risk strings so the
// literal-scan score crosses the advisory threshold.
class ApiConfig {
  static const apiKey = "AKIAABCDEFGHIJKLMNOP";
  static const authToken = "sk-proj-abcdefghijklmnopqrstuvwxyz123456";
  static const endpoint = "https://api.internal.example.com/v1/payments";
  static const statusReady = "ready";
  static const statusFailed = "failed";

  bool isReady(String status) {
    return status == "ready";
  }
}
