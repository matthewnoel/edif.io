<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { gs, updateRoomSettings } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import EditRoomView from '$lib/components/views/EditRoomView.svelte';

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

<EditRoomView
	{gameModes}
	bind:gameMode={selectedGameMode}
	bind:matchDuration
	bind:gameOptions={gameOptionValues}
	errorMessage={gs.errorMessage}
	canSubmit={gs.phase === 'ingame'}
	ready={initialized && isHost}
	onback={backToRoom}
	onupdate={updateRoom}
/>
