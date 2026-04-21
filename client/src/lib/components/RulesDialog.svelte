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
	<div class="rules-card" role="dialog" aria-label="How to Play">
		<div class="card-header">
			<h2>How to Play</h2>
			<CloseButton onclick={dismiss} ariaLabel="Dismiss rules" />
		</div>
		<ul>
			<li>Answer prompts correctly to <strong>grow</strong> your blob</li>
			<li>Wrong answers make you <strong>shrink</strong></li>
			<li>The <strong>biggest blob</strong> when time runs out wins</li>
		</ul>
		<h3>Power-Ups</h3>
		<ul>
			<li>💪 <strong>2x Points</strong> — doubles your growth for 30s</li>
			<li>🤪 <strong>Scrambled!</strong> — scrambles opponents' prompts for 20s</li>
			<li>🐢 <strong>Blue Shell!</strong> — instantly steal points from the leader</li>
			<li>🍕 <strong>Point Eater!</strong> — continuously drain the leader's points for 30s</li>
		</ul>
		<p class="hint">Trailing players get power-ups more often. The leader gets none!</p>
		<div class="footer">
			<Button label="Got it!" onclick={dismiss} />
		</div>
	</div>
{/if}

<style>
	.rules-card {
		background: white;
		border: 2px solid black;
		border-radius: 0.75rem;
		padding: 1.25rem 1.5rem 1rem;
		max-width: 420px;
		width: 100%;
		margin: 0 auto;
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

	h3 {
		margin: 0.35rem 0 0 0;
		font-size: 0.95rem;
	}

	ul {
		margin: 0;
		padding: 0 0 0 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.9rem;
	}

	.hint {
		margin: 0.25rem 0 0 0;
		font-size: 0.8rem;
		color: #6b7280;
	}

	.footer {
		display: flex;
		margin-top: 0.35rem;
	}
</style>
