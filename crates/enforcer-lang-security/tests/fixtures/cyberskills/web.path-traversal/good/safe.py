"""Path-traversal-safe file-serving patterns for validator fixtures.

Mirrors the vendor skill's own remediation guidance
(vendor/anthropic-cybersecurity-skills/skills/performing-directory-traversal-testing,
"Recommendation" section): allowlist known file names instead of trusting a
raw request-supplied path, and never let request input reach a file-op sink
directly.
"""

from pathlib import Path

from flask import Flask, send_from_directory

app = Flask(__name__)

UPLOAD_DIR = Path("/srv/uploads").resolve()

# SAFE: allowlist maps opaque keys to fixed on-disk file names — the raw
# request value never reaches a file-op sink.
ALLOWED_REPORTS = {
    "summary": "summary.pdf",
    "invoice": "invoice.pdf",
}


@app.route("/reports/<key>")
def reports(key: str):
    safe_name = ALLOWED_REPORTS.get(key)
    if safe_name is None:
        return "not found", 404
    return send_from_directory(UPLOAD_DIR, safe_name)


@app.route("/static-doc")
def static_doc():
    # SAFE: fixed literal path, no external input, no traversal sequence.
    with open(UPLOAD_DIR / "readme.txt") as handle:
        return handle.read()


def resolve_within_base(candidate: str) -> Path:
    """Canonicalize a candidate path and verify it stays under UPLOAD_DIR."""
    resolved = (UPLOAD_DIR / candidate).resolve()
    if UPLOAD_DIR not in resolved.parents and resolved != UPLOAD_DIR:
        raise ValueError("blocked path escape attempt")
    return resolved


@app.route("/document/<doc_key>")
def document(doc_key: str):
    # SAFE: the resolved path is checked against the allowed base directory
    # before it is ever opened.
    target = resolve_within_base(doc_key)
    with open(target) as handle:
        return handle.read()
