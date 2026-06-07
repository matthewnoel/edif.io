import { describe, it, expect } from 'vitest';

// Pseudo-localization coverage canary: every locale must define exactly the same
// keys as the base locale (`en`). Catalogs are auto-discovered, so adding a
// language requires no edit here — drop in `messages/<locale>.json` and it is
// checked automatically. If `en.json` gains a key and another catalog wasn't
// updated (or `l33t.json` wasn't regenerated via `npm run leetify`), this fails —
// the same guarantee `npm run leetify:check` enforces in CI for l33t, extended to
// every locale and caught in the unit suite.
const catalogs = import.meta.glob('../../../messages/*.json', { eager: true }) as Record<
	string,
	{ default: Record<string, unknown> }
>;

const localeOf = (path: string): string =>
	path
		.split('/')
		.pop()!
		.replace(/\.json$/, '');

const byLocale = new Map(
	Object.entries(catalogs).map(([path, mod]) => [localeOf(path), mod.default] as const)
);

describe('message catalog coverage', () => {
	const base = byLocale.get('en');

	it('has a base `en` catalog', () => {
		expect(base).toBeDefined();
	});

	const baseKeys = Object.keys(base ?? {}).sort();

	for (const [locale, catalog] of byLocale) {
		if (locale === 'en') continue;
		it(`${locale} defines exactly the same keys as en`, () => {
			expect(Object.keys(catalog).sort()).toEqual(baseKeys);
		});
	}
});
