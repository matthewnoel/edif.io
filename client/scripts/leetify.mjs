#!/usr/bin/env node
// Generates `messages/l33t.json` from `messages/en.json` by leetifying every
// string value. `l33t` is a real, committed message catalog consumed through
// the exact same Paraglide path future hand-authored languages use — the only
// difference is that its values are machine-generated here.
//
// Usage:
//   node scripts/leetify.mjs           # regenerate messages/l33t.json
//   node scripts/leetify.mjs --check   # fail (exit 1) if it is out of date
//
// This mirrors the `--check` convention of scripts/generate-agents-md.sh.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { leetify } from '../src/lib/i18n/leetify.js';

const here = dirname(fileURLToPath(import.meta.url));
const messagesDir = resolve(here, '../messages');
const enPath = resolve(messagesDir, 'en.json');
const l33tPath = resolve(messagesDir, 'l33t.json');

const en = JSON.parse(readFileSync(enPath, 'utf8'));

/** @type {Record<string, unknown>} */
const l33t = {};
for (const [key, value] of Object.entries(en)) {
	if (key === '$schema' || typeof value !== 'string') {
		// Preserve the schema reference and any non-string structural values.
		l33t[key] = value;
	} else {
		l33t[key] = leetify(value);
	}
}

const output = JSON.stringify(l33t, null, '\t') + '\n';

if (process.argv.includes('--check')) {
	let current = '';
	try {
		current = readFileSync(l33tPath, 'utf8');
	} catch {
		/* missing file counts as drift */
	}
	if (current !== output) {
		console.error('DRIFT: messages/l33t.json is out of date. Run: npm run leetify');
		process.exit(1);
	}
	console.log('messages/l33t.json is up to date.');
} else {
	writeFileSync(l33tPath, output);
	console.log('Generated messages/l33t.json');
}
