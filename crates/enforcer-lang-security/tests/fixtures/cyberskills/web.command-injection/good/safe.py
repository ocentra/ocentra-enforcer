"""Safe counterparts for CYBER-CMD-INJECT.1 (no command-injection sinks fire)."""
import os
import subprocess


def ping_host_safe(host):
    # No shell=True at all; argv list form avoids shell interpretation.
    subprocess.run(["ping", "-c", "1", host], shell=False)


def list_dir_safe():
    # Fully static string literal, no shell keyword at all.
    subprocess.run("ls -la /tmp")


def grep_log_safe():
    subprocess.run(["grep", "ERROR", "/var/log/app.log"])


def uname_safe():
    # Fully static string literal, no concatenation and no variable.
    os.system("uname -a")


def evaluate_constant_safe():
    # A fully static literal expression, no variable substitution.
    eval("2 * 2")


# Non-Python sinks kept as plain text for corpus parity; each uses only a
# fully static literal command, so none of them should be flagged.
NODE_EXEC_SAFE = "child_process.exec(\"ls -la\");"
NODE_EXEC_FILE_SAFE = "child_process.execFile(\"ls\", [\"-la\"]);"
RUBY_BACKTICK_SAFE = "`ls -la`"
RUBY_SYSTEM_SAFE = "system(\"ls -la\")"
JAVA_RUNTIME_SAFE = "Runtime.getRuntime().exec(\"ls -la\");"
