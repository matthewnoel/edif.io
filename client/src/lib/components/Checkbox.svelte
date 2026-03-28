<script lang="ts">
	import { randomColor } from '$lib/randomColor';

	interface Props {
		checked?: boolean;
		onchange?: (e: Event & { currentTarget: HTMLInputElement }) => void;
		disabled?: boolean;
	}

	let { checked = $bindable(false), onchange, disabled = false }: Props = $props();

	let accentColor = $state<string | undefined>(undefined);

	function activate() {
		accentColor = randomColor();
	}

	function deactivate() {
		accentColor = undefined;
	}
</script>

<input
	type="checkbox"
	bind:checked
	{onchange}
	{disabled}
	class="checkbox"
	style:--accent-color={accentColor}
	onpointerenter={activate}
	onpointerleave={deactivate}
	onfocusin={activate}
	onfocusout={deactivate}
/>

<style>
	.checkbox {
		appearance: none;
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid black;
		border-radius: 0.25rem;
		cursor: pointer;
		position: relative;
		background: transparent;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
	}

	.checkbox:checked {
		background: var(--accent-color, black);
		border-color: var(--accent-color, black);
	}

	.checkbox:checked::after {
		content: '';
		position: absolute;
		left: 50%;
		top: 50%;
		width: 0.3rem;
		height: 0.6rem;
		border: solid white;
		border-width: 0 2px 2px 0;
		transform: translate(-50%, -60%) rotate(45deg);
	}

	.checkbox:hover {
		border-color: var(--accent-color, black);
	}

	.checkbox:focus-visible {
		outline: 2px solid var(--accent-color, black);
		outline-offset: 2px;
	}

	.checkbox:disabled {
		opacity: 0.25;
		cursor: not-allowed;
	}
</style>
