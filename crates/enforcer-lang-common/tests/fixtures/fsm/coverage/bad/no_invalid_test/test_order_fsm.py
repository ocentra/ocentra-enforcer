# FAIL fixture for FSM-COVERAGE.1: an FSM transitions map is defined but
# no test asserts a raise on an illegal edge — score crosses the T2
# threshold.

transitions = {
    "PENDING": ["SHIPPED"],
    "SHIPPED": [],
}


def test_ship_from_pending():
    order = Order(status="PENDING")
    order.transition("SHIPPED")
    assert order.status == "SHIPPED"
