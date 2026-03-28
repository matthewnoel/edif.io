<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { randomColor } from '$lib/randomColor';

	interface Props extends HTMLInputAttributes {
		value?: string;
	}

	let { value = $bindable(''), ...rest }: Props = $props();

	let accentColor = $state<string | undefined>(undefined);

	let fillPercent = $derived(() => {
		const min = Number(rest.min ?? 0);
		const max = Number(rest.max ?? 100);
		const val = Number(value || min);
		if (max === min) return 0;
		return ((val - min) / (max - min)) * 100;
	});

	let fillColor = $derived(accentColor ?? 'black');

	function activate() {
		accentColor = randomColor();
	}

	function deactivate() {
		accentColor = undefined;
	}
</script>

<input
	type="range"
	bind:value
	class="range"
	style:--accent-color={accentColor}
	style:--fill-color={fillColor}
	style:--fill={`${fillPercent()}%`}
	onpointerenter={activate}
	onpointerleave={deactivate}
	onfocusin={activate}
	onfocusout={deactivate}
	{...rest}
/>

<style>
	.range {
		appearance: none;
		width: 100%;
		height: 0.25rem;
		background: linear-gradient(
			to right,
			var(--fill-color) 0%,
			var(--fill-color) var(--fill),
			#ccc var(--fill),
			#ccc 100%
		);
		border-radius: 0.125rem;
		outline: none;
		cursor: pointer;
	}

	.range:focus-visible {
		outline: 2px solid var(--accent-color, black);
		outline-offset: 4px;
	}

	/* Webkit (Chrome, Safari, Edge) */
	.range::-webkit-slider-thumb {
		appearance: none;
		width: 1.1rem;
		height: 1.1rem;
		border-radius: 50%;
		background: white;
		border: 2px solid var(--fill-color, black);
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
	}

	.range:hover::-webkit-slider-thumb {
		background: var(--accent-color, white);
	}

	/* Firefox */
	.range::-moz-range-thumb {
		width: 1.1rem;
		height: 1.1rem;
		border-radius: 50%;
		background: white;
		border: 2px solid var(--fill-color, black);
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
	}

	.range:hover::-moz-range-thumb {
		background: var(--accent-color, white);
	}

	.range::-moz-range-track {
		background: transparent;
	}

	.range::-moz-range-progress {
		background: transparent;
	}
</style>
