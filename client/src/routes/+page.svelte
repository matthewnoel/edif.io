<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount, onDestroy } from 'svelte';
	import { gs, connect, setOnWelcome, defaultWsUrl } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import Button from '$lib/components/Button.svelte';
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';
	import Select from '$lib/components/Select.svelte';
	import { ADJECTIVES, NOUNS, PALETTE } from '$lib/customization';

	const NAME_ADJ_KEY = 'edifio-player-name-adjective';
	const NAME_NOUN_KEY = 'edifio-player-name-noun';
	const COLOR_KEY = 'edifio-player-color';
	const readLS = (k: string): string =>
		typeof localStorage !== 'undefined' ? (localStorage.getItem(k) ?? '') : '';

	let wsUrl = $state('ws://localhost:4000/ws');
	let roomCodeInput = $state('');
	let selectedGameMode = $state('');
	let matchDuration = $state('60');
	let gameModes = $state<GameModeInfo[]>([]);
	let gameOptionValues = $state<Record<string, string>>({});
	let nameAdjective = $state(readLS(NAME_ADJ_KEY));
	let nameNoun = $state(readLS(NAME_NOUN_KEY));
	let playerColor = $state(readLS(COLOR_KEY));

	const playerName = $derived(nameAdjective && nameNoun ? `${nameAdjective} ${nameNoun}` : '');

	const adjectiveOptions = [
		{ value: '', label: '(random)' },
		...ADJECTIVES.map((a) => ({ value: a, label: a }))
	];
	const nounOptions = [
		{ value: '', label: '(random)' },
		...NOUNS.map((n) => ({ value: n, label: n }))
	];

	$effect(() => {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem(NAME_ADJ_KEY, nameAdjective);
		localStorage.setItem(NAME_NOUN_KEY, nameNoun);
		localStorage.setItem(COLOR_KEY, playerColor);
	});

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
			gameOptions: hasOptions ? gameOptionValues : undefined,
			playerName: playerName || undefined,
			playerColor: playerColor || undefined
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
			gameMode: selectedGameMode,
			playerName: playerName || undefined,
			playerColor: playerColor || undefined
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
		<section class="blob-customize" aria-label="Your blob">
			<div class="color-row" role="group" aria-label="Blob color">
				<button
					type="button"
					class="swatch random"
					class:selected={!playerColor}
					title="(random)"
					aria-label="Random color"
					aria-pressed={!playerColor}
					onclick={() => (playerColor = '')}>?</button
				>
				{#each PALETTE as color (color)}
					<button
						type="button"
						class="swatch"
						class:selected={playerColor === color}
						style:background-color={color}
						title={color}
						aria-label={`Color ${color}`}
						aria-pressed={playerColor === color}
						onclick={() => (playerColor = color)}
					></button>
				{/each}
			</div>
			<div class="name-row">
				<Select
					bind:value={nameAdjective}
					options={adjectiveOptions}
					aria-label="Blob name adjective"
				/>
				<Select bind:value={nameNoun} options={nounOptions} aria-label="Blob name noun" />
			</div>
		</section>
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
	.blob-customize {
		display: grid;
		gap: 0.5rem;
	}
	.color-row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	.swatch {
		width: 2rem;
		height: 2rem;
		border-radius: 50%;
		border: 2px solid black;
		padding: 0;
		cursor: pointer;
		background-color: transparent;
		font-size: 0.9rem;
		font-weight: bold;
	}
	.swatch.random {
		background-color: white;
		color: black;
	}
	.swatch.selected {
		outline: 2px solid black;
		outline-offset: 2px;
	}
	.name-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}
</style>
