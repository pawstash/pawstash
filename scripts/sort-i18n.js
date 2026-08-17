#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const LOCALES_DIR = path.join(__dirname, '..', 'src', 'lib', 'i18n', 'locales');

function sortObjectDeep(obj) {
  if (obj === null || typeof obj !== 'object' || Array.isArray(obj)) {
    return obj;
  }
  const sorted = {};
  for (const key of Object.keys(obj).sort((a, b) => a.localeCompare(b, 'en'))) {
    sorted[key] = sortObjectDeep(obj[key]);
  }
  return sorted;
}

function formatLocales() {
  if (!fs.existsSync(LOCALES_DIR)) {
    console.error(`Directory not found: ${LOCALES_DIR}`);
    process.exit(1);
  }

  const files = fs.readdirSync(LOCALES_DIR).filter(f => f.endsWith('.json'));

  for (const file of files) {
    const filePath = path.join(LOCALES_DIR, file);
    try {
      const raw = fs.readFileSync(filePath, 'utf8');
      const parsed = JSON.parse(raw);
      const sorted = sortObjectDeep(parsed);
      const formatted = JSON.stringify(sorted, null, 2) + '\n';
      fs.writeFileSync(filePath, formatted, 'utf8');
      console.log(`  sorted: ${file}`);
    } catch (err) {
      console.error(`  error: ${file}: ${err.message}`);
    }
  }
}

formatLocales();

