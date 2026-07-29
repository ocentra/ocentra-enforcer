"""Flask routes that build the template SOURCE from request input (SSTI).

Each handler below feeds a template-rendering sink a template TEXT string
that is itself constructed from untrusted request data, rather than passing
that data as a separate render argument. An attacker who controls the
interpolated value can inject template directives and, per the vendor SSTI
skill's Jinja2 exploitation steps, escalate to remote code execution.
"""
from flask import Flask, request, render_template_string
from jinja2 import Template
from mako.template import Template as MakoTemplate

app = Flask(__name__)


@app.route("/greet")
def greet():
    """Vulnerable: template text is an f-string built from user input."""
    name = request.args.get("name", "")
    return render_template_string(f"<h1>Hello {name}!</h1>")


@app.route("/echo")
def echo():
    """Vulnerable: template text is built via string concatenation."""
    message = request.args.get("message", "")
    return render_template_string("<p>" + message + "</p>")


@app.route("/notice")
def notice():
    """Vulnerable: template text is built via str.format()."""
    subject = request.args.get("subject", "")
    return render_template_string("<h2>{}</h2>".format(subject))


@app.route("/report")
def report():
    """Vulnerable: template text is built via %-formatting."""
    title = request.args.get("title", "")
    return render_template_string("<h3>%s</h3>" % title)


@app.route("/report2")
def report2():
    """Vulnerable: template text is built via %-formatting with a tuple."""
    title = request.args.get("title", "")
    subtitle = request.args.get("subtitle", "")
    return render_template_string("<h3>%s - %s</h3>" % (title, subtitle))


def render_email(body_var):
    """Vulnerable: jinja2 Template constructed from an f-string, rendered inline."""
    return Template(f"Dear customer, {body_var}").render(user=body_var)


def render_bio(user_bio):
    """Vulnerable: jinja2 Template constructed via concatenation, rendered inline."""
    return Template("<p>" + user_bio + "</p>").render()


def render_from_variable(raw_template):
    """Vulnerable: Template constructed directly from a bare, unquoted variable."""
    return Template(raw_template).render()


def render_mako_notice(user_input):
    """Vulnerable: mako Template (aliased MakoTemplate) built via concatenation."""
    return MakoTemplate("<p>" + user_input + "</p>").render()


def render_mako_fstring(user_input):
    """Vulnerable: mako Template (aliased MakoTemplate) built via f-string."""
    return MakoTemplate(f"<p>{user_input}</p>").render()
