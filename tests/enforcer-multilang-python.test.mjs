import assert from "node:assert/strict";
import test from "node:test";
import {
  makeProject,
  parseReport,
  pythonDoubleCall,
  pythonDoubleImport,
  run,
} from "./enforcer-multilang-test-support.mjs";

test("Python scanner catches strict source slop rules", () => {
  const project = makeProject({
    "src/service.py": `
from typing import Any, Optional, TypedDict, NamedTuple
from legacy import *
from pydantic import BaseModel
from dataclasses import dataclass
from datetime import datetime
from ..bad import value
import importlib
import os
import pickle
import requests
import subprocess
import time
import yaml

UserId = str
CACHE = {}

class UserShape(TypedDict):
    name: Optional[str]

class UserModel(BaseModel):
    name: str | None

@dataclass
class Point:
    x: int

class Pair(NamedTuple):
    left: str
    right: str

async def load(user_id: str, payload: dict[str, Any], headers={}):
    try:
        print(user_id)
        assert user_id
        now = datetime.now()
        eval("1 + 1")
        os.system("echo bad")
        os.getenv("TOKEN")
        pickle.loads(b"bad")
        yaml.load("x: 1")
        importlib.import_module("legacy")
        subprocess.run("echo bad", shell=True)
        time.sleep(1)
        send_async()
        asyncio.create_task(send_async())
        return requests.get(user_id).json()
    except Exception:
        pass

def load_bare(url: Any) -> dict[str, Any]:
    try:
        return {}
    except:
        return {}
`,
    "src/utils.py": "VALUE = 1\n",
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "python,common",
    "--files",
    "src/service.py",
    "src/utils.py",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of [
    "PY-4.1",
    "PY-4.2",
    "PY-4.3",
    "PY-4.4",
    "PY-4.5",
    "PY-4.6",
    "PY-4.7",
    "PY-4.8",
    "PY-4.9",
    "PY-4.10",
    "PY-4.11",
    "PY-4.12",
    "PY-4.13",
    "PY-4.14",
    "PY-4.15",
    "PY-4.16",
    "PY-4.17",
    "PY-4.18",
    "PY-4.19",
    "PY-4.20",
    "PY-4.21",
    "PY-4.22",
    "PY-4.23",
    "PY-4.24",
    "PY-4.25",
    "PY-4.26",
    "PY-4.27",
    "PY-4.28",
    "PY-4.29",
    "PY-4.30",
    "PY-4.31",
    "PY-4.32",
    "PY-4.33",
    "PY-4.34",
    "PY-4.35",
  ]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("Python scanner catches weak test assertions", () => {
  const project = makeProject({
    "tests/test_user.py": `
import requests
import time
${pythonDoubleImport}

import pytest

@pytest.mark.xfail(reason="not deterministic")
def test_expected_failure():
    assert True

def test_user(user):
    assert user

def test_empty():
    pass

def test_no_assert(monkeypatch):
    monkeypatch.setattr("pkg.value", 1)
    requests.get("https://example.test")
    time.sleep(1)
    ${pythonDoubleCall}
`,
    "tests/test_parser.py": `
def test_parser_valid():
    assert parse_user("1") == {"id": "1"}
`,
    "tests/test_exception.py": `
def test_error_path():
    assert validate_user("bad") is None
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "python,common",
    "--files",
    "tests/test_user.py",
    "tests/test_parser.py",
    "tests/test_exception.py",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  assert.equal(ids.has("PY-6.1"), true);
  assert.equal(ids.has("PY-6.2"), true);
  assert.equal(ids.has("PY-6.3"), true);
  assert.equal(ids.has("PY-6.4"), true);
  assert.equal(ids.has("PY-6.5"), true);
  assert.equal(ids.has("PY-6.6"), true);
  assert.equal(ids.has("PY-6.7"), true);
  assert.equal(ids.has("PY-6.8"), true);
  assert.equal(ids.has("PY-6.9"), true);
  assert.equal(ids.has("PY-6.10"), true);
});
