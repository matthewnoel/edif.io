<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount, onDestroy } from 'svelte';
	import { gs, connect, setOnWelcome, defaultWsUrl } from '$lib/game/connection.svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import Button from '$lib/components/Button.svelte';
	import Select from '$lib/components/Select.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import Checkbox from '$lib/components/Checkbox.svelte';
	import RangeInput from '$lib/components/RangeInput.svelte';

	let wsUrl = $state('ws://localhost:4000/ws');
	let roomCodeInput = $state('');
	let selectedGameMode = $state('');
	let matchDuration = $state('60');
	let gameModes = $state<GameModeInfo[]>([]);
	let gameOptionValues = $state<Record<string, string>>({});
	let code = $derived(roomCodeInput);

	let selectedMode = $derived(gameModes.find((m) => m.key === selectedGameMode));

	let visibleOptions = $derived.by(() => {
		if (!selectedMode?.options.length) return [];
		return selectedMode.options.filter((opt) => {
			if (!opt.visibleWhen) return true;
			return gameOptionValues[opt.visibleWhen.key] === opt.visibleWhen.value;
		});
	});

	const RANGE_PAIRS: [string, string][] = [
		['firstTermMinimumDigits', 'firstTermMaximumDigits'],
		['secondTermMinimumDigits', 'secondTermMaximumDigits']
	];

	function initOptionDefaults(mode: GameModeInfo | undefined): void {
		if (!mode || mode.options.length === 0) {
			gameOptionValues = {};
			return;
		}
		const defaults: Record<string, string> = {};
		for (const opt of mode.options) {
			defaults[opt.key] = gameOptionValues[opt.key] ?? String(opt.default);
		}
		gameOptionValues = defaults;
	}

	function handleRangeChange(key: string, value: string): void {
		gameOptionValues = { ...gameOptionValues, [key]: value };
		for (const [minKey, maxKey] of RANGE_PAIRS) {
			const min = parseInt(gameOptionValues[minKey] ?? '1');
			const max = parseInt(gameOptionValues[maxKey] ?? '1');
			if (key === minKey && min > max) {
				gameOptionValues = { ...gameOptionValues, [maxKey]: value };
			} else if (key === maxKey && max < min) {
				gameOptionValues = { ...gameOptionValues, [minKey]: value };
			}
		}
	}

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
					initOptionDefaults(modes[0]);
				}
			}
		} catch {
			/* server may not be running yet during dev */
		}
	});

	onDestroy(() => {
		setOnWelcome(null);
	});

	function handleGameModeChange(newMode: string): void {
		selectedGameMode = newMode;
		initOptionDefaults(gameModes.find((m) => m.key === newMode));
	}

	function createRoom(): void {
		const hasOptions = Object.keys(gameOptionValues).length > 0;
		connect(wsUrl, {
			gameMode: selectedGameMode,
			matchDurationSecs: parseInt(matchDuration) || 60,
			gameOptions: hasOptions ? gameOptionValues : undefined
		});
	}

	function joinRoom(): void {
		if (!code) {
			gs.errorMessage = 'Enter a room code to join';
			return;
		}
		if (code.length < 4) {
			gs.errorMessage = 'Room codes are 4 letters';
			return;
		}
		connect(wsUrl, {
			roomCode: code,
			gameMode: selectedGameMode
		});
	}
</script>

<main>
	<form
		class="pregame"
		onsubmit={(e) => {
			e.preventDefault();
			if (gs.phase === 'connecting') return;
			if (code) joinRoom();
			else createRoom();
		}}
	>
		<h1 class="shizuru-regular">edif.io</h1>
		{#if debugMode}
			<label>
				Server URL
				<TextInput
					bind:value={wsUrl}
					placeholder="ws://localhost:4000/ws"
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
					spellcheck="false"
				/>
			</label>
		{/if}
		{#if gameModes.length > 0}
			<label>
				<strong>Game Mode:</strong>
				<Select
					value={selectedGameMode}
					onchange={(e) => handleGameModeChange(e.currentTarget.value)}
					options={gameModes.map((m) => ({ value: m.key, label: m.label }))}
				/>
			</label>
		{/if}
		{#if visibleOptions.length}
			{#each visibleOptions as opt (opt.key)}
				{#if opt.type === 'select'}
					<label>
						<strong>{opt.label}:</strong>
						<Select
							value={gameOptionValues[opt.key] ?? opt.default}
							onchange={(e) => {
								gameOptionValues = { ...gameOptionValues, [opt.key]: e.currentTarget.value };
							}}
							options={opt.choices.map((c) => ({ value: c.value, label: c.label }))}
						/>
					</label>
				{:else if opt.type === 'range'}
					<label>
						<strong>{opt.label} ({gameOptionValues[opt.key] ?? opt.default}):</strong>
						<RangeInput
							min={opt.min}
							max={opt.max}
							step={opt.step}
							value={gameOptionValues[opt.key] ?? String(opt.default)}
							oninput={(e) => handleRangeChange(opt.key, e.currentTarget.value)}
						/>
					</label>
				{:else if opt.type === 'toggle'}
					<label class="toggle">
						<Checkbox
							checked={gameOptionValues[opt.key] === 'true'}
							onchange={(e) => {
								gameOptionValues = {
									...gameOptionValues,
									[opt.key]: String(e.currentTarget.checked)
								};
							}}
						/>
						<strong>{opt.label}</strong>
					</label>
				{/if}
			{/each}
		{/if}
		<label>
			<strong>Match Duration in Seconds:</strong>
			<TextInput
				bind:value={matchDuration}
				type="number"
				min="5"
				placeholder="60"
				autocomplete="off"
			/>
		</label>
		<label>
			<strong>Room Code (optional):</strong>
			<TextInput
				value={roomCodeInput}
				oninput={(e) => {
					const el = e.currentTarget;
					el.value = el.value.replace(/[^a-zA-Z]/g, '').toUpperCase();
					roomCodeInput = el.value;
				}}
				placeholder="ABCD"
				maxlength={4}
				pattern={'[A-Z]{4}'}
				autocomplete="off"
				autocorrect="off"
				autocapitalize="characters"
				spellcheck="false"
			/>
		</label>
		<button type="submit" hidden aria-hidden="true"></button>
		<div class="buttons">
			<Button
				label="Create Room"
				onclick={createRoom}
				disabled={gs.phase === 'connecting' || !!code}
			/>
			<Button label="Join Room" onclick={joinRoom} disabled={gs.phase === 'connecting' || !code} />
		</div>
		{#if gs.errorMessage}
			<p class="error">{gs.errorMessage}</p>
		{/if}
		{#if debugMode}
			<p class="meta">socket: {gs.socketState}</p>
			{#if gs.lastSocketDetail}
				<p class="meta">{gs.lastSocketDetail}</p>
			{/if}
		{/if}
	</form>
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

	label {
		display: grid;
		gap: 0.25rem;
		font-size: 0.92rem;
	}

	label.toggle {
		grid-template-columns: auto 1fr;
		align-items: center;
	}

	.buttons {
		display: flex;
		gap: 0.5rem;
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
