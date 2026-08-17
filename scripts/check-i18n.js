#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');
const LOCALES_DIR = path.join(ROOT_DIR, 'src', 'lib', 'i18n', 'locales');
const SRC_DIR = path.join(ROOT_DIR, 'src');

const args = process.argv.slice(2);
const isStrict = args.includes('--strict');
const showUnused = args.includes('--unused') || args.includes('-u');
const asJson = args.includes('--json');

const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

function flattenObject(obj, prefix = '') {
  const result = {};
  for (const [key, val] of Object.entries(obj)) {
    const fullPath = prefix ? `${prefix}.${key}` : key;
    if (val !== null && typeof val === 'object' && !Array.isArray(val)) {
      Object.assign(result, flattenObject(val, fullPath));
    } else {
      result[fullPath] = val;
    }
  }
  return result;
}

function extractPlaceholders(str) {
  if (typeof str !== 'string') return [];
  const matches = str.match(/\{([a-zA-Z0-9_]+)\}/g);
  return matches ? matches.map(m => m.slice(1, -1)).sort() : [];
}

function findSourceFiles(dir, extensions = ['.svelte', '.ts', '.js']) {
  let files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== 'node_modules' && entry.name !== 'locales') {
        files = files.concat(findSourceFiles(fullPath, extensions));
      }
    } else if (entry.isFile() && extensions.includes(path.extname(entry.name))) {
      files.push(fullPath);
    }
  }
  return files;
}

function extractKeysFromCode(files) {
  const codeKeys = new Map();
  const tCallRegex = /(?:i18n\.)?t\s*\(\s*([^,\n)]+)/g;
  const stringLiteralRegex = /['"`]([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)+)['"`]/g;

  for (const file of files) {
    const content = fs.readFileSync(file, 'utf8');
    const lines = content.split('\n');
    const relPath = path.relative(ROOT_DIR, file).replace(/\\/g, '/');

    lines.forEach((line, idx) => {
      const lineNum = idx + 1;
      let match;
      while ((match = tCallRegex.exec(line)) !== null) {
        const firstArg = match[1];
        let strMatch;
        while ((strMatch = stringLiteralRegex.exec(firstArg)) !== null) {
          const key = strMatch[1];
          if (!codeKeys.has(key)) {
            codeKeys.set(key, new Set());
          }
          codeKeys.get(key).add(`${relPath}:${lineNum}`);
        }
      }
    });
  }

  return codeKeys;
}

function runCheck() {
  if (!fs.existsSync(LOCALES_DIR)) {
    console.error(`Locales directory not found: ${LOCALES_DIR}`);
    process.exit(1);
  }

  const localeFiles = fs.readdirSync(LOCALES_DIR).filter(f => f.endsWith('.json'));
  if (localeFiles.length === 0) {
    console.error(`No .json locale files found in ${LOCALES_DIR}`);
    process.exit(1);
  }

  const locales = {};
  const flattenedLocales = {};
  const allKeysSet = new Set();

  for (const file of localeFiles) {
    const localeName = path.basename(file, '.json');
    const fullPath = path.join(LOCALES_DIR, file);
    try {
      const content = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
      locales[localeName] = content;
      const flat = flattenObject(content);
      flattenedLocales[localeName] = flat;
      for (const k of Object.keys(flat)) {
        allKeysSet.add(k);
      }
    } catch (err) {
      console.error(`Error parsing JSON in ${file}: ${err.message}`);
      process.exit(1);
    }
  }

  const allKeys = Array.from(allKeysSet).sort();
  const sourceFiles = findSourceFiles(SRC_DIR);
  const codeKeysMap = extractKeysFromCode(sourceFiles);

  const missingInLocale = {};
  const emptyInLocale = {};
  const placeholderMismatches = [];

  for (const localeName of Object.keys(locales)) {
    missingInLocale[localeName] = [];
    emptyInLocale[localeName] = [];
    const flat = flattenedLocales[localeName];

    for (const key of allKeys) {
      if (!(key in flat)) {
        missingInLocale[localeName].push(key);
      } else {
        const val = flat[key];
        if (typeof val === 'string' && val.trim() === '') {
          emptyInLocale[localeName].push(key);
        }
      }
    }
  }

  for (const key of allKeys) {
    const placeholdersPerLocale = {};
    for (const [localeName, flat] of Object.entries(flattenedLocales)) {
      if (key in flat && typeof flat[key] === 'string') {
        placeholdersPerLocale[localeName] = extractPlaceholders(flat[key]);
      }
    }

    const entries = Object.entries(placeholdersPerLocale);
    if (entries.length > 1) {
      const [firstLocale, firstVars] = entries[0];
      for (let i = 1; i < entries.length; i++) {
        const [otherLocale, otherVars] = entries[i];
        const firstStr = firstVars.join(',');
        const otherStr = otherVars.join(',');
        if (firstStr !== otherStr) {
          placeholderMismatches.push({
            key,
            localeA: firstLocale,
            varsA: firstVars,
            localeB: otherLocale,
            varsB: otherVars,
          });
          break;
        }
      }
    }
  }

  const missingInTranslations = [];
  for (const [codeKey, refs] of codeKeysMap.entries()) {
    if (!allKeysSet.has(codeKey)) {
      missingInTranslations.push({ key: codeKey, refs: Array.from(refs) });
    }
  }

  const unusedKeys = [];
  for (const key of allKeys) {
    if (!codeKeysMap.has(key)) {
      unusedKeys.push(key);
    }
  }

  let hasErrors = false;
  for (const keys of Object.values(missingInLocale)) {
    if (keys.length > 0) hasErrors = true;
  }
  for (const keys of Object.values(emptyInLocale)) {
    if (keys.length > 0) hasErrors = true;
  }
  if (missingInTranslations.length > 0) {
    hasErrors = true;
  }

  if (asJson) {
    console.log(JSON.stringify({
      totalKeys: allKeys.length,
      locales: Object.keys(locales),
      missingInLocale,
      emptyInLocale,
      placeholderMismatches,
      missingInTranslations,
      unusedKeys,
      hasErrors,
    }, null, 2));
    process.exit(hasErrors ? 1 : 0);
  }

  console.log(`\ni18n status: ${allKeys.length} keys across ${Object.keys(locales).join(', ')}`);

  for (const [localeName, flat] of Object.entries(flattenedLocales)) {
    const count = Object.keys(flat).length;
    const missing = missingInLocale[localeName].length;
    const empty = emptyInLocale[localeName].length;

    let status = `${c.green}ok (${count}/${allKeys.length})${c.reset}`;
    if (missing > 0 || empty > 0) {
      status = `${c.red}missing ${missing}, empty ${empty}${c.reset}`;
    }
    console.log(`  • ${localeName.padEnd(4)} : ${status}`);
  }

  if (missingInTranslations.length > 0) {
    console.log(`\n${c.red}Keys missing in translation files (${missingInTranslations.length}):${c.reset}`);
    for (const item of missingInTranslations) {
      console.log(`  ${c.red}-${c.reset} ${item.key}`);
      for (const ref of item.refs.slice(0, 3)) {
        console.log(`    ${c.gray}-> ${ref}${c.reset}`);
      }
    }
  }

  for (const [localeName, keys] of Object.entries(missingInLocale)) {
    if (keys.length > 0) {
      console.log(`\n${c.red}Keys missing in ${localeName} (${keys.length}):${c.reset}`);
      for (const k of keys.slice(0, 15)) {
        console.log(`  - ${k}`);
      }
      if (keys.length > 15) {
        console.log(`  ... +${keys.length - 15} more`);
      }
    }
  }

  for (const [localeName, keys] of Object.entries(emptyInLocale)) {
    if (keys.length > 0) {
      console.log(`\n${c.yellow}Empty keys in ${localeName} (${keys.length}):${c.reset}`);
      for (const k of keys) {
        console.log(`  - ${k}`);
      }
    }
  }

  if (placeholderMismatches.length > 0) {
    console.log(`\n${c.yellow}Placeholder mismatches (${placeholderMismatches.length}):${c.reset}`);
    for (const item of placeholderMismatches) {
      console.log(`  ${item.key}: ${item.localeA}={${item.varsA.join(',')}} vs ${item.localeB}={${item.varsB.join(',')}}`);
    }
  }

  if (unusedKeys.length > 0 && showUnused) {
    console.log(`\n${c.gray}Keys not referenced in code (${unusedKeys.length}):${c.reset}`);
    for (const k of unusedKeys) {
      console.log(`  ${c.gray}• ${k}${c.reset}`);
    }
  }

  if (hasErrors) {
    console.log(`\n${c.red}i18n check failed.${c.reset}\n`);
    process.exit(1);
  } else if (isStrict && (unusedKeys.length > 0 || placeholderMismatches.length > 0)) {
    console.log(`\n${c.yellow}i18n strict check failed on warnings.${c.reset}\n`);
    process.exit(1);
  } else {
    console.log(`\n${c.green}i18n check passed.${c.reset}\n`);
    process.exit(0);
  }
}

runCheck();

