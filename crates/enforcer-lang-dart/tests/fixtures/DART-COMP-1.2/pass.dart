// PASS fixture for DART-COMP-1.2: {super.key} is the first constructor param.
class OrderCard extends StatelessWidget {
  const OrderCard({super.key, required this.order});

  final Order order;

  Widget build(BuildContext context) => const Placeholder();
}
