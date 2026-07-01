import { spawn } from 'node:child_process';

await runCommand('node', ['scripts/test/activity-parent-assistant-runtime-proof.mjs']);
console.log('parent-assistant-provider-routing-proof-ok');

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: process.cwd(), stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code ?? 'unknown'}`));
    });
  });
}
