from flask import Flask

app = Flask(__name__)


@app.route("/widgets")
def list_widgets():
    return []


def test_list_widgets_returns_list():
    assert list_widgets() == []
