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
  red: '\x1b[31m',
  gray: '\x1b[90m',
};

const rawArg = process.argv[2];

if (!rawArg || rawArg === '--help' || rawArg === '-h') {
  console.log(`Usage: pnpm version:set <version>`);
  console.log(`Example: pnpm version:set 1.0.0`);
  process.exit(rawArg ? 0 : 1);
}

const newVersion = rawArg.startsWith('v') ? rawArg.slice(1) : rawArg;
const semverRegex = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

if (!semverRegex.test(newVersion)) {
  console.error(`Invalid SemVer format '${rawArg}' (expected e.g. 1.0.0, 1.0.0-pre1)`);
  process.exit(1);
}

const changes = [];

const pkgPath = path.join(ROOT_DIR, 'package.json');
if (fs.existsSync(pkgPath)) {
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
  const oldVer = pkg.version;
  pkg.version = newVersion;
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
  changes.push({ file: 'package.json', oldVer, newVer: newVersion });
}

const pkgLockPath = path.join(ROOT_DIR, 'package-lock.json');
if (fs.existsSync(pkgLockPath)) {
  const pkgLock = JSON.parse(fs.readFileSync(pkgLockPath, 'utf8'));
  const oldVer = pkgLock.version;
  pkgLock.version = newVersion;
  if (pkgLock.packages && pkgLock.packages['']) {
    pkgLock.packages[''].version = newVersion;
  }
  fs.writeFileSync(pkgLockPath, JSON.stringify(pkgLock, null, 2) + '\n', 'utf8');
  changes.push({ file: 'package-lock.json', oldVer, newVer: newVersion });
}

const cargoTomlPath = path.join(ROOT_DIR, 'src-tauri', 'Cargo.toml');
if (fs.existsSync(cargoTomlPath)) {
  let content = fs.readFileSync(cargoTomlPath, 'utf8');
  const match = content.match(/(\[package\][\s\S]*?version\s*=\s*")([^"]+)(")/);
  if (match) {
    const oldVer = match[2];
    content = content.replace(/(\[package\][\s\S]*?version\s*=\s*")[^"]+(")/, `$1${newVersion}$2`);
    fs.writeFileSync(cargoTomlPath, content, 'utf8');
    changes.push({ file: 'src-tauri/Cargo.toml', oldVer, newVer: newVersion });
  }
}

const tauriConfPath = path.join(ROOT_DIR, 'src-tauri', 'tauri.conf.json');
if (fs.existsSync(tauriConfPath)) {
  const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
  const oldVer = tauriConf.version;
  tauriConf.version = newVersion;
  fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n', 'utf8');
  changes.push({ file: 'src-tauri/tauri.conf.json', oldVer, newVer: newVersion });
}

console.log(`\nVersion updated to ${c.bold}${newVersion}${c.reset}:`);
for (const change of changes) {
  console.log(`  • ${change.file.padEnd(25)} ${c.gray}${change.oldVer}${c.reset} -> ${c.green}${change.newVer}${c.reset}`);
}
console.log();

