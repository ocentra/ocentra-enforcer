# FAIL fixture for FSM-TERMINAL.1: a terminal state (CLOSED) is given an
# outgoing edge instead of mapping to an empty list.

transitions = {
    "OPEN": ["CLOSED"],
    "CLOSED": ["REOPENED"],
}
