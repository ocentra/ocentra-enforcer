"""Session/cache loader that only ever parses data-only formats."""

import json

import yaml


def load_session(raw_cookie_json):
    # JSON has no code-execution gadget chain: it can only produce plain
    # data structures, never arbitrary objects.
    return json.loads(raw_cookie_json)


def load_cached_blob(raw_blob_json):
    return json.loads(raw_blob_json)


def load_config(raw_yaml_text):
    # SafeLoader restricts construction to plain Python types (dict, list,
    # str, int, ...), so no gadget chain can be reached from the document.
    return yaml.safe_load(raw_yaml_text)


def load_config_explicit_loader(raw_yaml_text):
    return yaml.load(raw_yaml_text, Loader=yaml.SafeLoader)
