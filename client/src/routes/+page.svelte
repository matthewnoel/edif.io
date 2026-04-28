<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount, onDestroy } from 'svelte';
	import { gs, connect, setOnWelcome, defaultWsUrl } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import Button from '$lib/components/Button.svelte';
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';

	let wsUrl = $state('ws://localhost:4000/ws');
	let roomCodeInput = $state('');
	let selectedGameMode = $state('');
	let matchDuration = $state('60');
	let gameModes = $state<GameModeInfo[]>([]);
	let gameOptionValues = $state<Record<string, string>>({});

	onMount(async () => {
		wsUrl = defaultWsUrl();
		setOnWelcome((roomCode) => {
			goto(resolve(`/room/${roomCode}`));
		});

		try {
			const res = await fetch('/api/game-modes');
			if (res.ok) {
				const modes: GameModeInfo[] = await res.json();
				gameModes = modes;
				if (modes.length > 0 && !selectedGameMode) {
					selectedGameMode = modes[0].key;
					const defaults: Record<string, string> = {};
					for (const opt of modes[0].options) {
						defaults[opt.key] = String(opt.default);
					}
					gameOptionValues = defaults;
				}
			}
		} catch {
			/* server may not be running yet during dev */
		}
	});

	onDestroy(() => {
		setOnWelcome(null);
	});

	function createRoom(): void {
		const hasOptions = Object.keys(gameOptionValues).length > 0;
		connect(wsUrl, {
			gameMode: selectedGameMode,
			matchDurationSecs: parseInt(matchDuration) || 60,
			gameOptions: hasOptions ? gameOptionValues : undefined
		});
	}

	function joinRoom(): void {
		if (!roomCodeInput) {
			gs.errorMessage = 'Enter a room code to join';
			return;
		}
		if (roomCodeInput.length < 4) {
			gs.errorMessage = 'Room codes are 4 letters';
			return;
		}
		connect(wsUrl, {
			roomCode: roomCodeInput,
			gameMode: selectedGameMode
		});
	}

	function handleSubmit(): void {
		if (gs.phase === 'connecting') return;
		if (roomCodeInput) joinRoom();
		else createRoom();
	}
</script>

<main>
	<div class="pregame">
		<h1 class="shizuru-regular">edif.io</h1>
		<GameSetupForm
			modes={gameModes}
			bind:gameMode={selectedGameMode}
			bind:matchDuration
			bind:gameOptions={gameOptionValues}
			bind:roomCode={roomCodeInput}
			bind:wsUrl
			showRoomCodeInput
			showServerUrl={debugMode}
			onsubmit={handleSubmit}
		>
			{#snippet buttons()}
				<Button
					label="Create Room"
					onclick={createRoom}
					disabled={gs.phase === 'connecting' || !!roomCodeInput}
				/>
				<Button
					label="Join Room"
					onclick={joinRoom}
					disabled={gs.phase === 'connecting' || !roomCodeInput}
				/>
			{/snippet}
		</GameSetupForm>
		{#if gs.errorMessage}
			<p class="error">{gs.errorMessage}</p>
		{/if}
		{#if debugMode}
			<p class="meta">socket: {gs.socketState}</p>
			{#if gs.lastSocketDetail}
				<p class="meta">{gs.lastSocketDetail}</p>
			{/if}
		{/if}
	</div>
</main>

<style>
	h1 {
		font-size: 4rem;
		text-align: center;
		margin: 1rem 0 0 0;
	}
	main {
		height: 100vh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: stretch;
	}
	.pregame {
		width: 100%;
		max-width: 460px;
		margin: 0 auto;
		padding: 0.5rem 1.25rem 10rem 1.25rem;
		display: grid;
		gap: 0.75rem;
	}

	.meta {
		margin: 0;
		font-size: 0.8rem;
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
