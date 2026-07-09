export function parseArgs(argv) {
  const args = {
    list: false,
    writeProof: null,
    shard: null,
    only: null,
    noRun: false,
    quiet: true,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--list") args.list = true;
    else if (arg === "--no-run") args.noRun = true;
    else if (arg === "--verbose") args.quiet = false;
    else if (arg === "--write-proof") args.writeProof = nextValue(argv, ++index, arg);
    else if (arg === "--shard") args.shard = parseShard(nextValue(argv, ++index, arg));
    else if (arg === "--only") args.only = nextValue(argv, ++index, arg);
    else throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function nextValue(argv, index, flag) {
  const value = argv.at(index);
  if (!value) throw new Error(`${flag} requires a value`);
  return value;
}

function parseShard(raw) {
  const parts = raw.split("/");
  const index = Number.parseInt(parts.at(0) ?? "", 10);
  const total = Number.parseInt(parts.at(1) ?? "", 10);
  if (!validShard(index, total)) throw new Error("--shard must use 1-based N/M");
  return { index, total };
}

function validShard(index, total) {
  return Number.isSafeInteger(index) && Number.isSafeInteger(total) && index >= 1 && total >= 1 && index <= total;
}
