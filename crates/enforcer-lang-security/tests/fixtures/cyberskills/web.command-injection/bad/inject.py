"""Fixtures for CYBER-CMD-INJECT.1 command-injection sinks (multi-language)."""
import os
import subprocess


def ping_host(host):
    # subprocess with shell=True is flagged unconditionally, even though the
    # command text below also happens to concatenate a variable in.
    subprocess.run("ping -c 1 " + host, shell=True)


def ping_host_literal():
    # ANY shell=True is flagged, even with a fully static command string.
    subprocess.call("ls -la", shell=True)


def list_dir(user_dir):
    # os.system with string concatenation.
    os.system("ls -la " + user_dir)


def grep_log(pattern):
    # os.system with an f-string.
    os.system(f"grep {pattern} /var/log/app.log")


def run_raw_command(cmd):
    # os.system with a bare variable.
    os.system(cmd)


def evaluate_expression(user_expr):
    # eval of a non-literal expression.
    eval(user_expr)


def execute_payload(payload):
    # exec of a non-literal expression.
    exec(payload)


# Non-Python sinks harvested for the same rule, kept as plain text so the
# per-line scanner can exercise the Node/Ruby/Java branches from this one
# fixture file.
NODE_EXEC_SNIPPET = "child_process.exec(`ls ${dir}`);"
NODE_EXEC_SYNC_SNIPPET = "child_process.execSync(\"rm -rf \" + target);"
RUBY_BACKTICK_SNIPPET = "`rm -rf #{path}`"
RUBY_SYSTEM_SNIPPET = "system(\"cat #{filename}\")"
JAVA_RUNTIME_SNIPPET = "Runtime.getRuntime().exec(\"cat \" + filename);"
