// FAIL fixture for DART-PERF-1.1: ListView(children: ...map...) over a dynamic collection.
class OrderList extends StatelessWidget {
  Widget build(BuildContext context) {
    return ListView(children: items.map((i) => Text(i.name)).toList());
  }
}
