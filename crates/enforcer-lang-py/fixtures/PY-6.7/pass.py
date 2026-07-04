def test_poll(fake_clock: "FakeClock") -> None:
    fake_clock.advance(seconds=1)
    assert poll() == "ready"
