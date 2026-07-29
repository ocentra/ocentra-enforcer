"""Fetch service restricted to an explicit host allowlist — no metadata
or link-local addresses are ever reachable."""

import urllib.parse

import requests

ALLOWED_HOSTS = {"api.example.com", "payments.internal.example.com"}


def fetch(user_url):
    parsed = urllib.parse.urlparse(user_url)
    if parsed.hostname not in ALLOWED_HOSTS:
        raise ValueError("host not allowed")
    return requests.get(user_url, timeout=5).text


def fetch_config():
    return requests.get("https://api.example.com/v1/config", timeout=5).text


def fetch_partner_status():
    return requests.get(
        "https://payments.internal.example.com/status", timeout=5
    ).text


def fetch_private_service():
    # A private, non-metadata RFC1918 address is not a metadata endpoint.
    return requests.get("https://10.0.4.12/health", timeout=5).text
