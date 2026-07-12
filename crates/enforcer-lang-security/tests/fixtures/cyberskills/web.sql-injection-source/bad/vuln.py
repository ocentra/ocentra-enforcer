"""Vulnerable data-access layer: SQL statements assembled by string
construction instead of parameterized queries. Every function below
splices a caller-supplied value directly into the SQL text before it
reaches the DB driver, so an attacker who controls that value can change
the statement's meaning (CYBER-SQLI-SOURCE.1).
"""

import sqlite3

import MySQLdb


def get_user_by_id(conn: sqlite3.Connection, user_id: str):
    """f-string interpolation directly into the SQL text."""
    cursor = conn.cursor()
    cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")
    return cursor.fetchone()


def get_appointment(conn: sqlite3.Connection, appointment_id: str):
    """String concatenation directly into the SQL text (the healthcare
    portal scenario from the vendor skill's Output Format section)."""
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM appointments WHERE id = " + appointment_id)
    return cursor.fetchall()


def find_account_by_email(conn: MySQLdb.Connection, email: str):
    """`%` string-formatting operator applied directly to the SQL text,
    as opposed to passing the value as a bound parameter."""
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM accounts WHERE email = '%s'" % email)
    return cursor.fetchone()


def update_order_status(conn: MySQLdb.Connection, order_id: str, status: str):
    """`.format()` chained directly onto the SQL string literal."""
    cursor = conn.cursor()
    cursor.execute("UPDATE orders SET status = '{}' WHERE id = {}".format(status, order_id))


def bulk_delete_sessions(conn: sqlite3.Connection, session_ids: list):
    """`.executemany()` fed an f-string-built DELETE statement."""
    cursor = conn.cursor()
    for session_id in session_ids:
        cursor.executemany(f"DELETE FROM sessions WHERE id = {session_id}", [])
