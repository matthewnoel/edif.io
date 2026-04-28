<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { gs, updateRoomSettings } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import Button from '$lib/components/Button.svelte';
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';

	let gameModes = $state<GameModeInfo[]>([]);
	let selectedGameMode = $state('');
	let matchDuration = $state('60');
	let gameOptionValues = $state<Record<string, string>>({});
	let initialized = $state(false);

	const code = $derived(page.params.code ?? '');

	let isHost = $derived(
		gs.room != null && gs.playerId != null && gs.playerId === gs.room.hostPlayerId
	);

	function backToRoom(): void {
		goto(resolve(`/room/${code}`));
	}

	onMount(async () => {
		if (gs.phase !== 'ingame' || !gs.room) {
			goto(resolve('/'));
			return;
		}
		if (!isHost) {
			backToRoom();
			return;
		}

		try {
			const res = await fetch('/api/game-modes');
			if (res.ok) {
				gameModes = await res.json();
			}
		} catch {
			/* server may not be running yet during dev */
		}

		if (gs.room) {
			selectedGameMode = gs.room.gameKey;
			matchDuration = String(gs.room.matchDurationSecs);
			const options: Record<string, string> = {};
			const raw = gs.room.gameOptions;
			if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
				for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
					options[key] = String(value);
				}
			}
			gameOptionValues = options;
		}
		initialized = true;
	});

	function updateRoom(): void {
		const hasOptions = Object.keys(gameOptionValues).length > 0;
		updateRoomSettings({
			gameMode: selectedGameMode,
			matchDurationSecs: parseInt(matchDuration) || 60,
			gameOptions: hasOptions ? gameOptionValues : undefined
		});
		backToRoom();
	}
</script>

<main>
	<div class="back">
		<Button label="⬅" onclick={backToRoom} />
	</div>
	<div class="edit">
		<h1 class="shizuru-regular">Edit Room</h1>
		{#if initialized && isHost}
			<GameSetupForm
				modes={gameModes}
				bind:gameMode={selectedGameMode}
				bind:matchDuration
				bind:gameOptions={gameOptionValues}
				onsubmit={updateRoom}
			>
				{#snippet buttons()}
					<Button label="Update Room" onclick={updateRoom} disabled={gs.phase !== 'ingame'} />
				{/snippet}
			</GameSetupForm>
			{#if gs.errorMessage}
				<p class="error">{gs.errorMessage}</p>
			{/if}
		{/if}
	</div>
</main>

<style>
	main {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: stretch;
	}

	h1 {
		font-size: 3rem;
		text-align: center;
		margin: 1rem 0 0 0;
	}

	.edit {
		width: 100%;
		max-width: 460px;
		margin: 0 auto;
		padding: 0.5rem 1.25rem 10rem 1.25rem;
		display: grid;
		gap: 0.75rem;
	}

	.back {
		position: fixed;
		top: 0.5rem;
		left: 0.5rem;
		z-index: 3;
		display: flex;
		gap: 0.5rem;
	}

	.back :global(.btn) {
		flex: 0 0 auto;
		padding: 0.4rem 0.7rem;
		font-size: 1.1rem;
		line-height: 1;
	}

	.error {
		background-color: transparent;
		color: red;
		padding: 0.5rem;
		border: 2px solid red;
		border-radius: 0.5rem;
		font-size: 0.8rem;
		margin: 0;
	}
</style>
