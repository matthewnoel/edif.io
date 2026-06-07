import { describe, it, expect } from 'vitest';
import en from '../../../messages/en.json';
import l33t from '../../../messages/l33t.json';

// Pseudo-localization coverage canary: every locale must define exactly the same
// keys as the base locale. If `en.json` gains a key and `l33t.json` wasn't
// regenerated (npm run leetify), this fails — the same guarantee `npm run
// leetify:check` enforces in CI, but caught here in the unit suite too.
describe('message catalog coverage', () => {
	it('l33t defines exactly the same keys as en', () => {
		const enKeys = Object.keys(en).sort();
		const l33tKeys = Object.keys(l33t).sort();
		expect(l33tKeys).toEqual(enKeys);
	});
});
