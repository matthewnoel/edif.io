<script lang="ts">
	import type { GameModeInfo } from '$lib/game/protocol';
	import Button from '$lib/components/Button.svelte';
	import GameSetupForm from '$lib/components/GameSetupForm.svelte';
	import { LEAVE_ICON } from '$lib/constants';
	import { m } from '$lib/paraglide/messages';

	interface Props {
		gameModes: GameModeInfo[];
		gameMode: string;
		matchDuration: string;
		gameOptions: Record<string, string>;
		errorMessage?: string;
		canSubmit?: boolean;
		ready?: boolean;
		onback: () => void;
		onupdate: () => void;
	}

	let {
		gameModes,
		gameMode = $bindable(''),
		matchDuration = $bindable('60'),
		gameOptions = $bindable({}),
		errorMessage = '',
		canSubmit = true,
		ready = false,
		onback,
		onupdate
	}: Props = $props();
</script>

<main>
	<div class="back">
		<Button label={LEAVE_ICON} onclick={onback} />
	</div>
	<div class="edit">
		<h1 class="shizuru-regular">{m.edit_room_title()}</h1>
		{#if ready}
			<GameSetupForm
				modes={gameModes}
				bind:gameMode
				bind:matchDuration
				bind:gameOptions
				onsubmit={onupdate}
			>
				{#snippet buttons()}
					<Button label={m.btn_update_room()} onclick={onupdate} disabled={!canSubmit} />
				{/snippet}
			</GameSetupForm>
			{#if errorMessage}
				<p class="error">{errorMessage}</p>
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
