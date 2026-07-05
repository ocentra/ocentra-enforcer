// PASS fixture for DART-SEC-1.1: key loaded from build-time environment.
class ApiConfig {
  static const apiKey = String.fromEnvironment('API_KEY');
}
