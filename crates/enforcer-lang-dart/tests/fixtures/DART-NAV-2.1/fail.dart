// FAIL fixture for DART-NAV-2.1: imperative Navigator.push (scored).
class OrderCard extends StatelessWidget {
  void onTap(BuildContext context) {
    Navigator.push(context, MaterialPageRoute(builder: (c) => const OrderPage()));
  }
}
