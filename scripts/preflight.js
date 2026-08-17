#!/usr/bin/env node

import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');

const isFast = process.argv.includes('--fast');
const isSkipRust = process.argv.includes('--skip-rust');

const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

function runStep(name, cmd, args = [], cwd = ROOT_DIR) {
  return new Promise((resolve, reject) => {
    const startTime = Date.now();
    process.stdout.write(`  ${name}... `);

    const fullCmd = args.length > 0 ? `${cmd} ${args.join(' ')}` : cmd;
    const child = spawn(fullCmd, {
      cwd,
      shell: true,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env, FORCE_COLOR: '1' }
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => { stdout += data.toString(); });
    child.stderr.on('data', (data) => { stderr += data.toString(); });

    child.on('close', (code) => {
      const duration = ((Date.now() - startTime) / 1000).toFixed(1);
      if (code === 0) {
        console.log(`${c.green}ok${c.reset} ${c.gray}(${duration}s)${c.reset}`);
        resolve({ stdout, stderr, duration });
      } else {
        console.log(`${c.red}fail${c.reset} ${c.gray}(${duration}s)${c.reset}`);
        reject({ code, stdout, stderr, cmd: `${cmd} ${args.join(' ')}` });
      }
    });

    child.on('error', (err) => {
      console.log(`${c.red}error${c.reset}`);
      reject({ code: 1, stdout, stderr: err.message, cmd: `${cmd} ${args.join(' ')}` });
    });
  });
}

async function main() {
  console.log(`\n${c.bold}Preflight verification${c.reset}`);

  const steps = [];

  steps.push({
    title: 'i18n validation',
    cmd: 'node',
    args: [path.join('scripts', 'check-i18n.js')],
  });

  if (!isSkipRust) {
    steps.push({
      title: 'Rust formatting (cargo fmt)',
      cmd: 'cargo',
      args: ['fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--all', '--', '--check'],
    });

    steps.push({
      title: 'Rust clippy (-D warnings)',
      cmd: 'cargo',
      args: ['clippy', '--manifest-path', 'src-tauri/Cargo.toml', '--workspace', '--all-targets', '--', '-D', 'warnings'],
    });

    if (!isFast) {
      steps.push({
        title: 'Rust tests (cargo test)',
        cmd: 'cargo',
        args: ['test', '--manifest-path', 'src-tauri/Cargo.toml', '--workspace'],
      });
    }
  }

  steps.push({
    title: 'Typecheck (svelte-check)',
    cmd: 'npx',
    args: ['svelte-check', '--tsconfig', './tsconfig.json'],
  });

  if (!isFast) {
    steps.push({
      title: 'Production build (vite build)',
      cmd: 'npx',
      args: ['vite', 'build'],
    });
  }

  const totalStartTime = Date.now();
  let completed = 0;

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    const stepNum = `[${i + 1}/${steps.length}]`;
    try {
      await runStep(`${stepNum} ${step.title}`, step.cmd, step.args);
      completed++;
    } catch (err) {
      console.log(`\n${c.red}${c.bold}Preflight check failed at: ${step.title}${c.reset}`);
      console.log(`${c.gray}Command:${c.reset} ${err.cmd}\n`);
      if (err.stdout && err.stdout.trim()) {
        console.log(`${c.yellow}STDOUT:${c.reset}\n${err.stdout}`);
      }
      if (err.stderr && err.stderr.trim()) {
        console.log(`${c.red}STDERR:${c.reset}\n${err.stderr}`);
      }
      process.exit(1);
    }
  }

  const totalDuration = ((Date.now() - totalStartTime) / 1000).toFixed(1);
  console.log(`\n${c.green}Preflight passed (${completed}/${steps.length} steps) in ${totalDuration}s${c.reset}\n`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

