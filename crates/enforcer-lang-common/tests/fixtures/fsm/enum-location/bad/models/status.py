# FAIL fixture for FSM-ENUMLOC.1: a status/role/type enum class outside
# `enums/`, or not inheriting a typed StrEnum base.
# (path marker: models/status.py)

from enum import Enum


class Status(Enum):
    PENDING = "pending"
    SHIPPED = "shipped"
