"""Toy fetch service that forwards a user-supplied URL — vulnerable to SSRF
against cloud instance metadata endpoints across every major provider."""

import requests


def fetch(user_url):
    # No allowlist, no scheme/host validation: an attacker can point
    # user_url at any internal or metadata address.
    return requests.get(user_url, timeout=5).text


def aws_imds_credentials():
    return requests.get(
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/"
    ).text


def aws_ecs_task_metadata():
    return requests.get("http://169.254.170.2/v2/credentials").text


def gcp_service_account_token():
    headers = {"Metadata-Flavor": "Google"}
    return requests.get(
        "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token",
        headers=headers,
    ).text


def alibaba_ram_credentials():
    return requests.get(
        "http://100.100.100.200/latest/meta-data/ram/security-credentials/"
    ).text


def ec2_ipv6_imds():
    return requests.get("http://[fd00:ec2::254]/latest/meta-data/").text
