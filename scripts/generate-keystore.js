#!/usr/bin/env node

import { execSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');
const KEYSTORE_PATH = path.join(ROOT_DIR, 'pawstash-release.keystore');

const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

function generateKeystore() {
  if (fs.existsSync(KEYSTORE_PATH)) {
    console.log(`Keystore already exists at: ${KEYSTORE_PATH}`);
  } else {
    const password = crypto.randomBytes(18).toString('base64').replace(/[^a-zA-Z0-9]/g, 'X') + '9!';
    const alias = 'pawstash';

    console.log(`Generating new Android keystore...`);

    const dname = 'CN=Pawstash, OU=Release, O=Pawstash, L=Unknown, ST=Unknown, C=US';
    const keytoolCmd = `keytool -genkeypair -v -keystore "${KEYSTORE_PATH}" -alias "${alias}" -keyalg RSA -keysize 2048 -validity 10000 -storepass "${password}" -keypass "${password}" -dname "${dname}"`;

    try {
      execSync(keytoolCmd, { stdio: 'pipe' });
      const credsPath = path.join(ROOT_DIR, 'keystore-credentials.txt');
      fs.writeFileSync(credsPath, `KEYSTORE_FILE=pawstash-release.keystore\nALIAS=${alias}\nPASSWORD=${password}\n`, 'utf8');
      console.log(`Saved credentials to: ${credsPath}`);
    } catch (err) {
      console.error(`Error generating keystore: ${err.message}`);
      process.exit(1);
    }
  }

  const keystoreBuffer = fs.readFileSync(KEYSTORE_PATH);
  const base64Keystore = keystoreBuffer.toString('base64');

  let password = 'YOUR_KEYSTORE_PASSWORD';
  const credsPath = path.join(ROOT_DIR, 'keystore-credentials.txt');
  if (fs.existsSync(credsPath)) {
    const lines = fs.readFileSync(credsPath, 'utf8').split('\n');
    for (const l of lines) {
      if (l.startsWith('PASSWORD=')) password = l.slice('PASSWORD='.length).trim();
    }
  }

  const base64File = path.join(ROOT_DIR, 'keystore-base64.txt');
  fs.writeFileSync(base64File, base64Keystore, 'utf8');

  console.log(`\nGitHub Actions Secrets:`);
  console.log(`  ANDROID_KEYSTORE_PASSWORD: ${password}`);
  console.log(`  ANDROID_KEY_PASSWORD:      ${password}`);
  console.log(`  ANDROID_KEY_ALIAS:         pawstash`);
  console.log(`  ANDROID_KEYSTORE_BASE64:    saved to ${base64File}\n`);
}

generateKeystore();

