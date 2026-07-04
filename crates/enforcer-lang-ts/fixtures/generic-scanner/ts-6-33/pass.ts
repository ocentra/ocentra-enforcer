import { ProcessRunner } from "./harness/process-runner";

export function run(command: string, runner: ProcessRunner): void {
  runner.run(command);
}
