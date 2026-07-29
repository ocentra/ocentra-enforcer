// PASS fixture for DART-SEC-1.4: default certificate verification kept.
class DefaultHttpOverrides extends HttpOverrides {
  HttpClient createHttpClient(SecurityContext? context) {
    return HttpClient(context: context);
  }
}
