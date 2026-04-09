<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { randomColor } from '$lib/randomColor';

	interface Props extends HTMLInputAttributes {
		value?: string;
		el?: HTMLInputElement | null;
		inlineButtonLabel?: string;
		inlineButtonOnclick?: () => void;
	}

	let {
		value = $bindable(''),
		el = $bindable(null),
		inlineButtonLabel,
		inlineButtonOnclick,
		...rest
	}: Props = $props();

	let accentColor = $state<string | undefined>(undefined);
	let btnAccentColor = $state<string | undefined>(undefined);

	function activate() {
		accentColor ??= randomColor();
	}

	function deactivate() {
		accentColor = undefined;
	}

	function activateBtn() {
		btnAccentColor ??= randomColor();
	}

	function deactivateBtn() {
		btnAccentColor = undefined;
	}
</script>

<div class="wrapper" class:has-button={!!inlineButtonLabel}>
	<input
		bind:this={el}
		bind:value
		class="input"
		style:--accent-color={accentColor}
		onpointerenter={activate}
		onpointerleave={deactivate}
		onfocusin={activate}
		onfocusout={deactivate}
		{...rest}
	/>
	{#if inlineButtonLabel}
		<button
			class="inline-btn"
			style:--accent-color={btnAccentColor}
			onclick={inlineButtonOnclick}
			onpointerenter={activateBtn}
			onpointerleave={deactivateBtn}
			onfocusin={activateBtn}
			onfocusout={deactivateBtn}
			disabled={rest.disabled}
			aria-label={inlineButtonLabel}>{inlineButtonLabel}</button
		>
	{/if}
</div>

<style>
	.wrapper {
		position: relative;
		display: flex;
		flex: 1;
		min-width: 0;
	}

	.input {
		flex: 1;
		background: transparent;
		color: inherit;
		border: 2px solid black;
		border-radius: 0.5rem;
		padding: 0.6rem;
		font-size: inherit;
		min-width: 0;
		transition:
			border-color 0.15s ease,
			outline-color 0.15s ease;
	}

	.has-button .input {
		padding-right: 3.5rem;
	}

	.input:hover {
		border-color: var(--accent-color, black);
	}

	.input:focus-visible {
		outline: 2px solid var(--accent-color, black);
		outline-offset: 2px;
	}

	.inline-btn {
		position: absolute;
		right: 0.35rem;
		top: 0.35rem;
		bottom: 0.35rem;
		padding: 0 0.6rem;
		background: var(--accent-color, black);
		color: white;
		border: none;
		border-radius: 0.3rem;
		font-size: 0.85rem;
		font-weight: 700;
		cursor: pointer;
		transition: background 0.15s ease;
	}

	.inline-btn:disabled {
		opacity: 0.25;
		cursor: not-allowed;
	}
</style>
