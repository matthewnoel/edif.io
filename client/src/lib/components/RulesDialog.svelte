<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import CloseButton from '$lib/components/CloseButton.svelte';

	const STORAGE_KEY = 'rulesDialogDismissed';

	let dismissed = $state(
		typeof localStorage !== 'undefined' && localStorage.getItem(STORAGE_KEY) === '1'
	);

	function dismiss() {
		dismissed = true;
		localStorage.setItem(STORAGE_KEY, '1');
	}
</script>

{#if !dismissed}
	<div
		class="overlay"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) dismiss();
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				e.preventDefault();
				dismiss();
			}
		}}
	>
		<div class="rules-card" role="dialog" aria-label="How to Play" tabindex="-1">
			<div class="card-header">
				<h2>How to Play</h2>
				<CloseButton onclick={dismiss} ariaLabel="Dismiss rules" />
			</div>
			<ul>
				<li>Answer correctly to <strong>grow</strong> your blob</li>
				<li>Wrong answers make you <strong>shrink</strong></li>
				<li>Players will receieve <strong>power-ups</strong> during the match</li>
				<li>The <strong>biggest blob</strong> wins once the timer runs out</li>
			</ul>
			<div class="footer">
				<Button label="Got it!" onclick={dismiss} />
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 10;
		background: rgba(0, 0, 0, 0.4);
		display: grid;
		place-items: center;
		padding: 1rem;
		box-sizing: border-box;
	}

	.rules-card {
		background: white;
		border: 2px solid black;
		border-radius: 0.75rem;
		padding: 1.25rem 1.5rem 1rem;
		max-width: 420px;
		width: 100%;
		text-align: left;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.card-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	h2 {
		margin: 0;
		font-size: 1.1rem;
	}

	ul {
		margin: 0;
		padding: 0 0 0 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.9rem;
	}

	.footer {
		display: flex;
		margin-top: 0.35rem;
	}
</style>
