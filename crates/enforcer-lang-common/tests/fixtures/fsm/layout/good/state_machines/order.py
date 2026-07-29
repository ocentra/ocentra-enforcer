# PASS fixture for FSM-LAYOUT.1: the transitions map lives in
# `state_machines/`, enums in `enums/` — canonical layout.
# (path marker: state_machines/order.py)

transitions = {
    "pending": ["shipped"],
    "shipped": [],
}
