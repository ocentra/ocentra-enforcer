// FAIL fixture for DART-SEC-1.2: auth token written to SharedPreferences.
class AuthStorage {
  Future<void> save(SharedPreferences prefs, String token) async {
    await prefs.setString('auth_token', token);
  }
}

class SharedPreferences {
  Future<void> setString(String key, String value) async {}
}
