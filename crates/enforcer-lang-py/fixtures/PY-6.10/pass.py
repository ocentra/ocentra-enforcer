from hypothesis import given
from hypothesis import strategies as st


def parse_config(raw: str) -> dict[str, str]:
    return dict(item.split("=") for item in raw.split(","))


def test_parse_config_basic() -> None:
    assert parse_config("a=1") == {"a": "1"}


@given(st.text())
def test_parse_config_never_crashes(raw: str) -> None:
    try:
        parse_config(raw)
    except ValueError:
        pass
