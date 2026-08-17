#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');

const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  gray: '\x1b[90m',
};

const cleanAll = process.argv.includes('--all') || process.argv.includes('-a');

const dirsToClean = [
  path.join(ROOT_DIR, 'dist'),
  path.join(ROOT_DIR, '.svelte-kit'),
  path.join(ROOT_DIR, 'node_modules', '.vite'),
];

if (cleanAll) {
  dirsToClean.push(path.join(ROOT_DIR, 'src-tauri', 'target'));
}

console.log(`Cleaning build artifacts and cache...`);

for (const dir of dirsToClean) {
  const relPath = path.relative(ROOT_DIR, dir);
  if (fs.existsSync(dir)) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
      console.log(`  ${c.green}ok${c.reset} removed ${relPath}`);
    } catch (err) {
      console.warn(`  ${c.yellow}warn${c.reset} could not remove ${relPath}: ${err.message}`);
    }
  }
}

console.log(`Done.\n`);

