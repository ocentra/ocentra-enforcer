// FAIL fixture for DART-SEC-1.4: TLS certificate verification disabled.
class InsecureHttpOverrides extends HttpOverrides {
  HttpClient createHttpClient(SecurityContext? context) {
    final client = HttpClient(context: context);
    client.badCertificateCallback = (cert, host, port) => true;
    return client;
  }
}
