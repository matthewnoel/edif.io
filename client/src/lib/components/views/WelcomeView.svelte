<script lang="ts">
	import type { GameModeInfo } from '$lib/game/protocol';
	import Button from '$lib/components/Button.svelte';
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';

	interface Props {
		gameModes: GameModeInfo[];
		gameMode: string;
		matchDuration: string;
		gameOptions: Record<string, string>;
		roomCode: string;
		wsUrl: string;
		showServerUrl?: boolean;
		errorMessage?: string;
		connecting?: boolean;
		debug?: boolean;
		socketState?: string;
		lastSocketDetail?: string;
		oncreate: () => void;
		onjoin: () => void;
		onsubmit: () => void;
	}

	let {
		gameModes,
		gameMode = $bindable(''),
		matchDuration = $bindable('60'),
		gameOptions = $bindable({}),
		roomCode = $bindable(''),
		wsUrl = $bindable(''),
		showServerUrl = false,
		errorMessage = '',
		connecting = false,
		debug = false,
		socketState = '',
		lastSocketDetail = '',
		oncreate,
		onjoin,
		onsubmit
	}: Props = $props();
</script>

<main>
	<div class="pregame">
		<h1 class="shizuru-regular">edif.io</h1>
		<GameSetupForm
			modes={gameModes}
			bind:gameMode
			bind:matchDuration
			bind:gameOptions
			bind:roomCode
			bind:wsUrl
			showRoomCodeInput
			{showServerUrl}
			{onsubmit}
		>
			{#snippet buttons()}
				<Button label="Create Room" onclick={oncreate} disabled={connecting || !!roomCode} />
				<Button label="Join Room" onclick={onjoin} disabled={connecting || !roomCode} />
			{/snippet}
		</GameSetupForm>
		{#if errorMessage}
			<p class="error">{errorMessage}</p>
		{/if}
		{#if debug}
			<p class="meta">socket: {socketState}</p>
			{#if lastSocketDetail}
				<p class="meta">{lastSocketDetail}</p>
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
