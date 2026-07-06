def test_retry_backoff_schedules_the_expected_delay():
    clock = FakeClock(start=0.0)
    decision = retry_with_backoff(clock=clock)
    assert decision.delay_seconds == 0.5
