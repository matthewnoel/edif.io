<script lang="ts">
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';
	import Button from '$lib/components/Button.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import { defaultOptionValues } from '$lib/dev/fixtures';

	interface Props {
		modes: GameModeInfo[];
		initialGameMode?: string;
		showRoomCodeInput?: boolean;
		showServerUrl?: boolean;
		submitLabel?: string;
	}

	let {
		modes,
		initialGameMode,
		showRoomCodeInput = false,
		showServerUrl = false,
		submitLabel = 'Create Room'
	}: Props = $props();

	// svelte-ignore state_referenced_locally
	const firstMode = modes.find((m) => m.key === initialGameMode) ?? modes[0];

	let gameMode = $state(firstMode?.key ?? '');
	let matchDuration = $state('60');
	let gameOptions = $state<Record<string, string>>(firstMode ? defaultOptionValues(firstMode) : {});
	let roomCode = $state('');
	let wsUrl = $state('ws://localhost:4000/ws');
</script>

<div class="frame">
	<GameSetupForm
		{modes}
		bind:gameMode
		bind:matchDuration
		bind:gameOptions
		bind:roomCode
		bind:wsUrl
		{showRoomCodeInput}
		{showServerUrl}
		onsubmit={() => {}}
	>
		{#snippet buttons()}
			<Button label={submitLabel} onclick={() => {}} disabled={!!roomCode} />
			{#if showRoomCodeInput}
				<Button label="Join Room" onclick={() => {}} disabled={!roomCode} />
			{/if}
		{/snippet}
	</GameSetupForm>

	<pre class="state">{JSON.stringify(
			{ gameMode, matchDuration, roomCode, gameOptions },
			null,
			2
		)}</pre>
</div>

<style>
	.frame {
		max-width: 460px;
		margin: 0 auto;
		padding: 1.5rem 1.25rem;
		display: grid;
		gap: 1rem;
	}

	.state {
		font-size: 0.75rem;
		background: #f3f4f6;
		border-radius: 0.4rem;
		padding: 0.6rem 0.8rem;
		overflow: auto;
		margin: 0;
	}
</style>
