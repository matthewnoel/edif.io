<script lang="ts">
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
		<button class="dismiss-btn" onclick={dismiss} aria-label="Dismiss rules">✕</button>
		<h2>How to Play</h2>
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
		<button class="got-it-btn" onclick={dismiss}>Got it!</button>
	</div>
{/if}

<style>
	.rules-card {
		position: relative;
		background: white;
		border: 2px solid black;
		border-radius: 0.75rem;
		padding: 1.25rem 1.5rem 1rem;
		max-width: 420px;
		margin: 0 auto;
		text-align: left;
		box-sizing: border-box;
	}

	h2 {
		margin: 0 0 0.75rem 0;
		font-size: 1.1rem;
	}

	h3 {
		margin: 0.85rem 0 0.5rem 0;
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
		margin: 0.75rem 0 0.85rem 0;
		font-size: 0.8rem;
		color: #6b7280;
	}

	.dismiss-btn {
		position: absolute;
		top: 0.6rem;
		right: 0.7rem;
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		color: #6b7280;
		padding: 0.2rem 0.3rem;
		border-radius: 0.25rem;
	}

	.dismiss-btn:hover {
		color: black;
		background: #f3f4f6;
	}

	.got-it-btn {
		display: block;
		width: 100%;
		background: none;
		border: 2px solid black;
		border-radius: 0.5rem;
		padding: 0.45rem 1rem;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 700;
	}

	.got-it-btn:hover {
		background: black;
		color: white;
	}
</style>
