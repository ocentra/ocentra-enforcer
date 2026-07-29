"""Deliberately vulnerable path-traversal / LFI sinks for validator fixtures.

Mirrors the scenarios from
vendor/anthropic-cybersecurity-skills/skills/performing-directory-traversal-testing
(Scenario 1: file-download traversal, Scenario 3: filter-bypass traversal)
but as static source patterns rather than live HTTP probes.
"""

import os
from flask import Flask, request, send_file

app = Flask(__name__)

UPLOAD_DIR = "/srv/uploads"


@app.route("/debug")
def debug_dump():
    # VULNERABLE: a literal `../` traversal sequence is concatenated
    # straight into the path passed to a file-op sink.
    handle = open(UPLOAD_DIR + "/../../etc/passwd", "r")
    return handle.read()


@app.route("/view")
def view():
    # VULNERABLE: request-derived filename concatenated straight into a
    # file-handling sink with no allowlist or canonicalization check.
    return send_file(UPLOAD_DIR + "/" + request.args.get("file"))


@app.route("/legacy")
def legacy_read():
    user_path = request.form.get("path")
    # VULNERABLE: the sink argument is a bare variable whose name is
    # obviously request-derived ("user_path" contains "user").
    with open(user_path) as handle:
        return handle.read()


@app.route("/encoded")
def encoded_dump():
    # VULNERABLE: URL-encoded traversal sequence reaches fopen()-style sink
    # on the same line as the call.
    return os.fopen(UPLOAD_DIR + "..%2f..%2f..%2fetc%2fpasswd")
