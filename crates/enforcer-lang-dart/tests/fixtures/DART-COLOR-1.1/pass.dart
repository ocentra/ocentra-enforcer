// PASS fixture for DART-COLOR-1.1: theme color used instead of a literal.
class OrderBadge extends StatelessWidget {
  Widget build(BuildContext context) {
    return Container(color: Theme.of(context).colorScheme.primary);
  }
}
