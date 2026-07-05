// FAIL fixture for DART-INITSTATE-1.1: data fetch kicked off from initState (scored).
class OrderListState extends State<OrderListPage> {
  void initState() {
    super.initState();
    fetchOrders().then((data) => setState(() => orders = data));
  }
}
