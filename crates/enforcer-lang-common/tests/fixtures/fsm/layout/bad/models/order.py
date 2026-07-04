# FAIL fixture for FSM-LAYOUT.1: a transitions map declared inside
# `models/`, not the canonical `state_machines/` location.
# (path marker: models/order.py)

transitions = {
    "pending": ["shipped"],
    "shipped": [],
}


class Order:
    status: str
