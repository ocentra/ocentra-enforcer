// FAIL fixture for DART-COMP-1.2: widget constructor missing {super.key}.
class OrderCard extends StatelessWidget {
  const OrderCard(this.order);

  final Order order;

  Widget build(BuildContext context) => const Placeholder();
}
