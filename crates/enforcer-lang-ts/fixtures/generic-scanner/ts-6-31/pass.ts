export async function wait(clock: FakeClock): Promise<void> {
  await clock.advanceBy(1000);
}
