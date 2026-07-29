// PASS fixture for DART-NAV-2.1: declarative GoRouter navigation with a named route.
class OrderCard extends StatelessWidget {
  void onTap(BuildContext context) {
    context.go('/orders');
  }
}
