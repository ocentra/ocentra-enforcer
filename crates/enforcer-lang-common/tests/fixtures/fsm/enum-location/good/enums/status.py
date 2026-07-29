# PASS fixture for FSM-ENUMLOC.1: the Status enum lives in `enums/` and
# inherits the typed StrEnum base.
# (path marker: enums/status.py)

from enum import StrEnum


class Status(StrEnum):
    PENDING = "pending"
    SHIPPED = "shipped"
