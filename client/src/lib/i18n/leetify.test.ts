import { describe, it, expect } from 'vitest';
import { leetify } from './leetify.js';

describe('leetify', () => {
	it('substitutes the known leet letters (case-insensitively)', () => {
		expect(leetify('adventure')).toBe('4dv3n7ur3');
		expect(leetify('aeiost')).toBe('431057');
		expect(leetify('Answer')).toBe('4n5w3r');
	});

	it('leaves unmapped letters and non-letters untouched', () => {
		expect(leetify('blob')).toBe('bl0b');
		expect(leetify('2x Points!')).toBe('2x P01n75!');
	});

	it('preserves {placeholder} tokens verbatim', () => {
		expect(leetify('{winner} wins the match')).toBe('{winner} w1n5 7h3 m47ch');
		expect(leetify('Player {id}')).toBe('Pl4y3r {id}');
	});

	it('preserves **bold** markers, leetifying the wrapped word', () => {
		expect(leetify('Answer correctly to **grow** your blob')).toBe(
			'4n5w3r c0rr3c7ly 70 **gr0w** y0ur bl0b'
		);
	});

	it('is idempotent on already-leetified text', () => {
		expect(leetify('h3ll0')).toBe('h3ll0');
	});
});
