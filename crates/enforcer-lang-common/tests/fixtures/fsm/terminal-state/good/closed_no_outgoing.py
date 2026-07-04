# PASS fixture for FSM-TERMINAL.1: the terminal state CLOSED maps to an
# empty list — no outgoing edge.

transitions = {
    "OPEN": ["CLOSED"],
    "CLOSED": [],
}
