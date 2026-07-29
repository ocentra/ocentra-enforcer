// Pass fixture for LIT-2.1 (universal literal-scan T2 advisory bridge).
// Clean equivalent: no hardcoded secrets/routes/magic-string comparisons,
// so the literal-scan score stays under the advisory threshold.
class Greeter {
  String greet(String name) {
    return 'Hello, $name!';
  }
}
