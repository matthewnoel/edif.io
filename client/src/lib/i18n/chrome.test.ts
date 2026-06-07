import { describe, it, expect } from 'vitest';
import { modeLabel, optionLabel, choiceLabel, inputPlaceholder } from './chrome';

// These run in the node test environment, where the locale resolves to the base
// locale (en), so a known key returns its English message and an unknown key
// falls back to the server-provided label.
describe('adapter chrome localization', () => {
	it('returns the catalog label for a known stable key', () => {
		expect(modeLabel('keyboarding', 'IGNORED FALLBACK')).toBe('Keyboarding');
	});

	it('maps hyphenated game keys to underscore message ids', () => {
		expect(modeLabel('state-abbreviations', 'IGNORED FALLBACK')).toBe('US State Abbreviations');
	});

	it('falls back to the server-provided label for unknown keys', () => {
		expect(modeLabel('made-up-game', 'Server Mode')).toBe('Server Mode');
		expect(optionLabel('made-up-game', 'whatever', 'Server Option')).toBe('Server Option');
		expect(choiceLabel('made-up-game', 'k', 'v', 'Server Choice')).toBe('Server Choice');
		expect(inputPlaceholder('made-up-game', 'Server Placeholder')).toBe('Server Placeholder');
	});
});
