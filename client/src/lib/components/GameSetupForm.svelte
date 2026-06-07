<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { GameModeInfo } from '$lib/game/protocol';
	import Select from '$lib/components/Select.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import Checkbox from '$lib/components/Checkbox.svelte';
	import RangeInput from '$lib/components/RangeInput.svelte';
	import { m } from '$lib/paraglide/messages';
	import { modeLabel, optionLabel, choiceLabel } from '$lib/i18n/chrome';

	interface Props {
		modes: GameModeInfo[];
		gameMode: string;
		matchDuration: string;
		gameOptions: Record<string, string>;
		roomCode?: string;
		wsUrl?: string;
		showRoomCodeInput?: boolean;
		showServerUrl?: boolean;
		onsubmit?: () => void;
		buttons: Snippet;
	}

	let {
		modes,
		gameMode = $bindable(''),
		matchDuration = $bindable('60'),
		gameOptions = $bindable({}),
		roomCode = $bindable(''),
		wsUrl = $bindable(''),
		showRoomCodeInput = false,
		showServerUrl = false,
		onsubmit,
		buttons
	}: Props = $props();

	const RANGE_PAIRS: [string, string][] = [
		['firstTermMinimumDigits', 'firstTermMaximumDigits'],
		['secondTermMinimumDigits', 'secondTermMaximumDigits']
	];

	let selectedMode = $derived(modes.find((m) => m.key === gameMode));

	let visibleOptions = $derived.by(() => {
		if (!selectedMode?.options.length) return [];
		return selectedMode.options.filter((opt) => {
			if (!opt.visibleWhen) return true;
			return gameOptions[opt.visibleWhen.key] === opt.visibleWhen.value;
		});
	});

	function initOptionDefaults(mode: GameModeInfo | undefined): void {
		if (!mode || mode.options.length === 0) {
			gameOptions = {};
			return;
		}
		const defaults: Record<string, string> = {};
		for (const opt of mode.options) {
			defaults[opt.key] = gameOptions[opt.key] ?? String(opt.default);
		}
		gameOptions = defaults;
	}

	function handleGameModeChange(newMode: string): void {
		gameMode = newMode;
		initOptionDefaults(modes.find((m) => m.key === newMode));
	}

	function handleRangeChange(key: string, value: string): void {
		gameOptions = { ...gameOptions, [key]: value };
		for (const [minKey, maxKey] of RANGE_PAIRS) {
			const min = parseInt(gameOptions[minKey] ?? '1');
			const max = parseInt(gameOptions[maxKey] ?? '1');
			if (key === minKey && min > max) {
				gameOptions = { ...gameOptions, [maxKey]: value };
			} else if (key === maxKey && max < min) {
				gameOptions = { ...gameOptions, [minKey]: value };
			}
		}
	}
</script>

<form
	class="setup"
	onsubmit={(e) => {
		e.preventDefault();
		onsubmit?.();
	}}
>
	{#if showServerUrl}
		<label>
			{m.label_server_url()}
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
	{#if modes.length > 0}
		<label>
			<strong>{m.label_game_mode()}</strong>
			<Select
				value={gameMode}
				onchange={(e) => handleGameModeChange(e.currentTarget.value)}
				options={modes.map((mode) => ({ value: mode.key, label: modeLabel(mode.key, mode.label) }))}
			/>
		</label>
	{/if}
	{#if visibleOptions.length}
		{#each visibleOptions as opt (opt.key)}
			{#if opt.type === 'select'}
				<label>
					<strong>{optionLabel(gameMode, opt.key, opt.label)}:</strong>
					<Select
						value={gameOptions[opt.key] ?? opt.default}
						onchange={(e) => {
							gameOptions = { ...gameOptions, [opt.key]: e.currentTarget.value };
						}}
						options={opt.choices.map((c) => ({
							value: c.value,
							label: choiceLabel(gameMode, opt.key, c.value, c.label)
						}))}
					/>
				</label>
			{:else if opt.type === 'range'}
				<label>
					<strong
						>{optionLabel(gameMode, opt.key, opt.label)} ({gameOptions[opt.key] ??
							opt.default}):</strong
					>
					<RangeInput
						min={opt.min}
						max={opt.max}
						step={opt.step}
						value={gameOptions[opt.key] ?? String(opt.default)}
						oninput={(e) => handleRangeChange(opt.key, e.currentTarget.value)}
					/>
				</label>
			{:else if opt.type === 'toggle'}
				<label class="toggle">
					<Checkbox
						checked={gameOptions[opt.key] === 'true'}
						onchange={(e) => {
							gameOptions = {
								...gameOptions,
								[opt.key]: String(e.currentTarget.checked)
							};
						}}
					/>
					<strong>{optionLabel(gameMode, opt.key, opt.label)}</strong>
				</label>
			{/if}
		{/each}
	{/if}
	<label>
		<strong>{m.label_match_duration()}</strong>
		<TextInput
			bind:value={matchDuration}
			type="number"
			min="5"
			placeholder="60"
			autocomplete="off"
		/>
	</label>
	{#if showRoomCodeInput}
		<label>
			<strong>{m.label_room_code_optional()}</strong>
			<TextInput
				value={roomCode}
				oninput={(e) => {
					const el = e.currentTarget;
					el.value = el.value.replace(/[^a-zA-Z]/g, '').toUpperCase();
					roomCode = el.value;
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
	{/if}
	<button type="submit" hidden aria-hidden="true"></button>
	<div class="buttons">
		{@render buttons()}
	</div>
</form>

<style>
	.setup {
		width: 100%;
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
</style>
