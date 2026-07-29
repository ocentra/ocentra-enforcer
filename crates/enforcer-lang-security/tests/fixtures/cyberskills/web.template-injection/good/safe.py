"""Flask routes that render templates safely.

Each handler below either renders a named template FILE (so untrusted data
flows through the engine's own escaping as a separate render argument) or a
fully static, literal template TEXT with no interpolation. The template
SOURCE itself is never constructed from a variable, so there is no
server-side template injection surface here.
"""
from flask import Flask, request, render_template, render_template_string
from jinja2 import Template

app = Flask(__name__)


@app.route("/profile")
def profile():
    """Safe: named template FILE; user data passed as a render kwarg."""
    name = request.args.get("name", "")
    return render_template("index.html", name=name)


@app.route("/dashboard")
def dashboard():
    """Safe: named template FILE with multiple kwargs."""
    user = request.args.get("user", "")
    return render_template("dashboard.html", user=user, active=True)


@app.route("/banner")
def banner():
    """Safe: static literal template string, no interpolation into the text."""
    return render_template_string("<h1>Hello</h1>")


@app.route("/promo")
def promo():
    """Safe: static literal template text that happens to contain a bare '%'."""
    return render_template_string("Save 20% today")


def render_email(body_var):
    """Safe: static template text; user data passed only as a render() kwarg."""
    return Template("Dear customer, {{ body }}").render(body=body_var)


def build_greeting(name):
    """Safe: an ordinary f-string that is never passed to a template renderer."""
    return f"Hello {name}, welcome back!"


def compile_static_handlebars_like(template_source):
    """Safe: a bare variable handed to a template compiler is not itself an
    interpolated template literal, and this file never calls one anyway;
    kept here to document the intended non-sink usage."""
    return template_source
