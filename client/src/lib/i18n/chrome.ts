import { m } from '$lib/paraglide/messages';

/**
 * Adapter "chrome" — game-mode labels, option-field labels, select-choice
 * labels and input placeholders — originates on the backend in English. It is
 * part of the viewer-language surface, so we localize it client-side keyed by
 * the stable identifiers the server already sends (`gameKey`, option `key`,
 * choice `value`), falling back to the English label the server provides for any
 * key we don't have a message for (e.g. a future adapter the frontend hasn't
 * translated yet, which then degrades gracefully to English).
 */
const messages = m as unknown as Record<
	string,
	((inputs?: Record<string, never>) => string) | undefined
>;

function translateOr(key: string, fallback: string): string {
	const fn = messages[key];
	return typeof fn === 'function' ? fn() : fallback;
}

// Stable game keys may contain hyphens (e.g. "state-abbreviations"); message
// identifiers use underscores.
const ident = (key: string): string => key.replace(/-/g, '_');

export function modeLabel(gameKey: string, fallback: string): string {
	return translateOr(`adapter_${ident(gameKey)}`, fallback);
}

export function inputPlaceholder(gameKey: string, fallback: string): string {
	return translateOr(`placeholder_${ident(gameKey)}`, fallback);
}

export function optionLabel(gameKey: string, optionKey: string, fallback: string): string {
	return translateOr(`opt_${ident(gameKey)}_${optionKey}`, fallback);
}

export function choiceLabel(
	gameKey: string,
	optionKey: string,
	value: string,
	fallback: string
): string {
	return translateOr(`choice_${ident(gameKey)}_${optionKey}_${value}`, fallback);
}
