/**
 * Leetspeak transform used to GENERATE the `l33t` message catalog from English.
 *
 * This is plain JS (not TS) on purpose: it is imported both by the Vitest test
 * suite and by the Node generator script (`scripts/leetify.mjs`), and the
 * generator must run on the CI Node version without any TypeScript loader.
 *
 * Only ASCII letters are substituted; everything else passes through. Two
 * constructs are preserved verbatim so generated messages keep working:
 *   - interpolation placeholders, e.g. `{winner}` — never transformed.
 *   - markdown bold markers `**...**` — the `*` are non-letters so they survive
 *     automatically; only the wrapped word is leetified.
 *
 * @param {string} input
 * @returns {string}
 */
export function leetify(input) {
	/** @type {Record<string, string>} */
	const map = { a: '4', e: '3', i: '1', o: '0', s: '5', t: '7' };
	return input
		.split(/(\{[^}]*\})/g)
		.map((segment, index) => {
			// Odd indices are the captured `{placeholder}` groups — keep verbatim.
			if (index % 2 === 1) return segment;
			return segment.replace(/[a-zA-Z]/g, (ch) => map[ch.toLowerCase()] ?? ch);
		})
		.join('');
}
