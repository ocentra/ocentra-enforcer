"""Fixed data-access layer: the same operations as vuln.py, but every
value reaches the DB driver as a bound parameter instead of being
spliced into the SQL text (the vendor skill's Output Format Remediation:
`$stmt = $pdo->prepare("SELECT * FROM appointments WHERE id = ?");
$stmt->execute([$_GET['id']]);`).
"""

import sqlite3

import MySQLdb

from myapp.models import Account


def get_user_by_id(conn: sqlite3.Connection, user_id: str):
    """`?` placeholder with the value passed as a separate parameter."""
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
    return cursor.fetchone()


def get_appointment(conn: sqlite3.Connection, appointment_id: str):
    """Same `?` placeholder pattern as the vendor skill's SECURE example."""
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM appointments WHERE id = ?", (appointment_id,))
    return cursor.fetchall()


def find_account_by_email(conn: MySQLdb.Connection, email: str):
    """`%s` placeholder with the value passed as a separate params tuple
    (the DB driver binds it — no `%` operator is applied to the string)."""
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM accounts WHERE email = %s", (email,))
    return cursor.fetchone()


def update_order_status(conn: MySQLdb.Connection, order_id: str, status: str):
    """Named placeholders with a separate params mapping."""
    cursor = conn.cursor()
    cursor.execute(
        "UPDATE orders SET status = %(status)s WHERE id = %(order_id)s",
        {"status": status, "order_id": order_id},
    )


def bulk_delete_sessions(conn: sqlite3.Connection, session_ids: list):
    """`.executemany()` with a static SQL string and a separate params
    sequence."""
    cursor = conn.cursor()
    cursor.executemany(
        "DELETE FROM sessions WHERE id = ?", [(sid,) for sid in session_ids]
    )


def find_accounts_by_status(status: str):
    """ORM query builder: no raw SQL string is ever assembled."""
    return Account.objects.filter(status=status)
