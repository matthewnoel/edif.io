<script lang="ts">
	import { randomColor } from '$lib/randomColor';

	interface Props {
		onclick?: () => void;
		ariaLabel?: string;
	}

	let { onclick, ariaLabel = 'Close' }: Props = $props();

	let accentColor = $state<string | undefined>(undefined);

	function activate() {
		accentColor = randomColor();
	}

	function deactivate() {
		accentColor = undefined;
	}
</script>

<button
	{onclick}
	type="button"
	aria-label={ariaLabel}
	class="close-btn"
	class:active={accentColor !== undefined}
	style:--accent-color={accentColor}
	onpointerenter={activate}
	onpointerleave={deactivate}
	onfocusin={activate}
	onfocusout={deactivate}
>
	<strong>✕</strong>
</button>

<style>
	.close-btn {
		background: var(--accent-color, transparent);
		color: var(--accent-color, black);
		border: 2px solid var(--accent-color, black);
		border-radius: 9999px;
		width: 2rem;
		height: 2rem;
		display: grid;
		place-items: center;
		cursor: pointer;
		font-size: inherit;
		padding: 0;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}

	.close-btn.active {
		color: white;
	}

	.close-btn:focus-visible {
		outline: 2px solid var(--accent-color, black);
		outline-offset: 2px;
	}
</style>
