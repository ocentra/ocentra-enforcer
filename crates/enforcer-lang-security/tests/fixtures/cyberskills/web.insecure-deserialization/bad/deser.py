"""Session/cache loader that deserializes attacker-controlled bytes."""

import pickle
import marshal
import yaml


def load_session(raw_cookie_bytes):
    # Classic pickle RCE sink: an attacker-crafted __reduce__ payload runs
    # arbitrary code the moment this line executes.
    return pickle.loads(raw_cookie_bytes)


def load_cached_blob(raw_blob):
    # marshal has no security guarantees for untrusted bytes either.
    return marshal.loads(raw_blob)


def load_config(raw_yaml_text):
    # Default Loader can instantiate arbitrary Python objects from the
    # document (e.g. !!python/object/apply:os.system [...]).
    return yaml.load(raw_yaml_text)
