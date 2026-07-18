/** Print coordination help without dispatching commands that can mutate ledger state. */
export async function runCoordinationCommandWithHelp(argv, runCommand) {
  if (argv.some((arg) => ["--help", "-h"].includes(arg))) {
    console.log(`Usage: ocentra-enforcer coordination ${argv[0]} [options]`);
    return;
  }
  await runCommand(argv);
}
