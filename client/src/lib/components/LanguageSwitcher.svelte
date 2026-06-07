<script lang="ts">
	import Select from '$lib/components/Select.svelte';
	import { m } from '$lib/paraglide/messages';
	import { locales, getLocale, setLocale } from '$lib/paraglide/runtime';

	// Each language names itself via a `lang_name_<locale>` message (the endonym,
	// e.g. "Español"), so adding a viewer language needs no change here — just the
	// new key in the catalogs. Fall back to the raw code if a name is missing.
	const names = m as unknown as Record<
		string,
		((inputs?: Record<string, never>) => string) | undefined
	>;

	function nameFor(locale: string): string {
		const fn = names[`lang_name_${locale.replace(/-/g, '_')}`];
		return typeof fn === 'function' ? fn() : locale;
	}

	// `setLocale` persists to localStorage and reloads the page, so the whole app
	// re-renders in the chosen language — no manual reactivity required.
	function choose(locale: string): void {
		setLocale(locale as (typeof locales)[number]);
	}
</script>

<label class="language">
	<span>{m.label_language()}</span>
	<Select
		value={getLocale()}
		onchange={(e) => choose(e.currentTarget.value)}
		options={locales.map((locale) => ({ value: locale, label: nameFor(locale) }))}
	/>
</label>

<style>
	.language {
		display: grid;
		gap: 0.25rem;
		font-size: 0.92rem;
	}
</style>
