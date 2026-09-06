#!/usr/bin/env node

import { execSync, spawn } from 'node:child_process';

const c = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
};

// Set up adb reverse tunnels for Vite dev server and HMR WebSocket
const ports = [1420, 1421];
for (const port of ports) {
  try {
    execSync(`adb reverse tcp:${port} tcp:${port}`, { stdio: 'ignore' });
    console.log(`${c.green}ok${c.reset} adb reverse tcp:${port} -> tcp:${port}`);
  } catch {
    console.warn(`${c.yellow}warn${c.reset} could not adb reverse tcp:${port} (device might not be connected yet)`);
  }
}

console.log(`${c.cyan}info${c.reset} starting tauri android dev with --host 127.0.0.1...\n`);

const extraArgs = process.argv.slice(2);
const args = ['tauri', 'android', 'dev', '--host', '127.0.0.1', ...extraArgs];

const child = spawn(process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm', args, {
  stdio: 'inherit',
  shell: true,
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
