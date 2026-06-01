<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount, onDestroy } from 'svelte';
	import { gs, connect, setOnWelcome, defaultWsUrl } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import WelcomeView from '$lib/components/views/WelcomeView.svelte';

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

<WelcomeView
	{gameModes}
	bind:gameMode={selectedGameMode}
	bind:matchDuration
	bind:gameOptions={gameOptionValues}
	bind:roomCode={roomCodeInput}
	bind:wsUrl
	showServerUrl={debugMode}
	errorMessage={gs.errorMessage}
	connecting={gs.phase === 'connecting'}
	debug={debugMode}
	socketState={gs.socketState}
	lastSocketDetail={gs.lastSocketDetail}
	oncreate={createRoom}
	onjoin={joinRoom}
	onsubmit={handleSubmit}
/>
