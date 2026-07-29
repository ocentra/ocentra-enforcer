// FAIL fixture for DART-COMP-1.1: two public widgets declared in one file.
class OrderCard extends StatelessWidget {
  const OrderCard({super.key});

  Widget build(BuildContext context) => const Placeholder();
}

class OrderBadge extends StatelessWidget {
  const OrderBadge({super.key});

  Widget build(BuildContext context) => const Placeholder();
}
