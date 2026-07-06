from time import monotonic


def test_retry_backoff_completes_within_budget():
    start = monotonic()
    retry_with_backoff()
    assert monotonic() - start <= 0.6
