// FAIL fixture for DART-COLOR-1.1: hardcoded color literal in build (scored).
class OrderBadge extends StatelessWidget {
  Widget build(BuildContext context) {
    return Container(color: const Color(0xFF00FF00));
  }
}
