<script lang="ts">
	import Select from '$lib/components/Select.svelte';
	import { m } from '$lib/paraglide/messages';
	import { locales, getLocale, setLocale } from '$lib/paraglide/runtime';

	// Display names for the viewer-language picker. Each language names itself.
	const LANGUAGE_NAMES: Record<string, () => string> = {
		en: m.lang_name_en,
		l33t: m.lang_name_l33t
	};

	function nameFor(locale: string): string {
		return (LANGUAGE_NAMES[locale] ?? (() => locale))();
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
