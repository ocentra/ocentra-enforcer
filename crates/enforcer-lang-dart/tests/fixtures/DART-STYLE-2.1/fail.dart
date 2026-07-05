// FAIL fixture for DART-STYLE-2.1: string concatenation instead of interpolation (scored).
class Greeter {
  String greet(String name) {
    return 'Hello ' + name;
  }
}
