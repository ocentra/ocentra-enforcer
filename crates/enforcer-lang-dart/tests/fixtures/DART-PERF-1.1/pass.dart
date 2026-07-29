// PASS fixture for DART-PERF-1.1: ListView.builder used for a dynamic list.
class OrderList extends StatelessWidget {
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: items.length,
      itemBuilder: (context, index) => Text(items[index].name),
    );
  }
}
