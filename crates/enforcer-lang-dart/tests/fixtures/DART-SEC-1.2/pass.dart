// PASS fixture for DART-SEC-1.2: token stored via flutter_secure_storage.
class AuthStorage {
  Future<void> save(String token) async {
    await const FlutterSecureStorage().write(key: 'auth_token', value: token);
  }
}

class FlutterSecureStorage {
  const FlutterSecureStorage();
  Future<void> write({required String key, required String value}) async {}
}
