# PASS fixture for FSM-COVERAGE.1: an FSM transitions map is defined AND a
# test asserts InvalidTransitionError on an illegal edge — score stays
# under threshold.

transitions = {
    "PENDING": ["SHIPPED"],
    "SHIPPED": [],
}


def test_ship_from_pending():
    order = Order(status="PENDING")
    order.transition("SHIPPED")
    assert order.status == "SHIPPED"


def test_reship_from_shipped_raises_invalid_transition():
    order = Order(status="SHIPPED")
    with pytest.raises(InvalidTransitionError):
        order.transition("SHIPPED")
