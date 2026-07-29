import { exec } from "child_process";

export function run(command: string): void {
  exec(command);
}
