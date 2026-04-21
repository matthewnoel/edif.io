<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { browser } from '$app/environment';
	import { onMount, onDestroy } from 'svelte';
	import {
		gs,
		connect,
		setOnDisconnect,
		handlePromptInput,
		submitPrompt,
		startMatch,
		rematch,
		socketStateLabel,
		defaultWsUrl,
		loadSession,
		loadRejoinToken,
		disconnect
	} from '$lib/game/connection.svelte';
	import { nextBlobLayout, type BlobLayout } from '$lib/game/sim';
	import type { PlayerSnapshot, PowerUpKind } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import Button from '$lib/components/Button.svelte';
	import PowerUpBadge from '$lib/components/PowerUpBadge.svelte';
	import RulesDialog from '$lib/components/RulesDialog.svelte';
	import TextInput from '$lib/components/TextInput.svelte';

	type PowerUpMeta = {
		emoji: string;
		label: string;
		affectsSelf: boolean;
		disablesInput: boolean;
	};

	const POWERUP_META: Record<PowerUpKind, PowerUpMeta> = {
		doublePoints: {
			emoji: '\u{1F4AA}',
			label: '2x Points',
			affectsSelf: true,
			disablesInput: false
		},
		scrambleFont: {
			emoji: '\u{1F92A}',
			label: 'Scrambled!',
			affectsSelf: false,
			disablesInput: false
		},
		scoreSteal: {
			emoji: '\u{1F422}',
			label: 'Blue Shell!',
			affectsSelf: true,
			disablesInput: false
		},
		ongoingScoreSteal: {
			emoji: '\u{1F355}',
			label: 'Point Eater!',
			affectsSelf: true,
			disablesInput: false
		}
	};

	const RING_CIRCUMFERENCE = 106.81;

	let arenaEl: HTMLDivElement | null = $state(null);
	let blobLayout: BlobLayout = $state({});
	let debugOpen = $state(false);
	let animationHandle = 0;
	let visualHeight = $state(0);
	let timerDisplayMs = $state<number | null>(null);
	let timerBaseMs = 0;
	let timerSyncedAt = 0;
	let powerupRingOffsets = $state<Record<number, number>>({});
	let effectTimers = $state<Record<string, { expiresAt: number; durationMs: number }>>({});
	let effectFractions = $state<Record<string, number>>({});
	let promptInputEl: HTMLInputElement | null = $state(null);
	let lastEffectTimerKey = '';
	let copyConfirmed = $state(false);
	let copyTimeout = 0;
	let powerUpToastTimeout = 0;

	let myActiveEffects = $derived([
		...new Map(
			(gs.room?.activePowerups ?? [])
				.filter((pu) => {
					if (pu.remainingMs <= 0) return false;
					const meta = POWERUP_META[pu.kind];
					return meta.affectsSelf
						? pu.sourcePlayerId === gs.playerId
						: pu.sourcePlayerId !== gs.playerId;
				})
				.map(
					(pu) =>
						[
							pu.kind,
							{ ...POWERUP_META[pu.kind], kind: pu.kind, durationMs: pu.durationMs }
						] as const
				)
		).values()
	]);

	let inputDisabled = $derived(myActiveEffects.some((e) => e.disablesInput));

	let promptScrambled = $derived(
		(gs.room?.activePowerups ?? []).some(
			(pu) => pu.kind === 'scrambleFont' && pu.sourcePlayerId !== gs.playerId && pu.remainingMs > 0
		)
	);

	let myColor = $derived(gs.room?.players.find((p) => p.id === gs.playerId)?.color ?? null);

	let myPendingPowerUps = $derived(gs.pendingPowerUps.filter((pu) => pu.playerId === gs.playerId));

	let otherPendingPowerUps = $derived(
		gs.pendingPowerUps
			.filter((pu) => pu.playerId !== gs.playerId)
			.map((pu) => {
				const player = gs.room?.players.find((p) => p.id === pu.playerId);
				return { ...pu, playerName: player?.name ?? '???', playerColor: player?.color ?? '#888' };
			})
	);

	function playerPowerUpEmojis(playerId: number): string {
		return [
			...new Set(
				(gs.room?.activePowerups ?? [])
					.filter((pu) => {
						if (pu.remainingMs <= 0) return false;
						const meta = POWERUP_META[pu.kind];
						return meta.affectsSelf
							? pu.sourcePlayerId === playerId
							: pu.sourcePlayerId !== playerId;
					})
					.map((pu) => POWERUP_META[pu.kind].emoji)
			)
		].join('');
	}

	function formatTimer(ms: number): string {
		const totalSeconds = Math.max(0, Math.ceil(ms / 1000));
		const m = Math.floor(totalSeconds / 60);
		const s = totalSeconds % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	$effect(() => {
		const serverMs = gs.room?.matchRemainingMs ?? null;
		if (serverMs != null) {
			if (serverMs !== timerBaseMs) {
				timerBaseMs = serverMs;
				timerSyncedAt = performance.now();
				timerDisplayMs = serverMs;
			}
		} else {
			timerDisplayMs = null;
			timerBaseMs = 0;
		}
	});

	$effect(() => {
		const powerups = gs.room?.activePowerups ?? [];
		const key = JSON.stringify(powerups);
		if (key === lastEffectTimerKey) return;
		lastEffectTimerKey = key;

		const now = performance.now();
		const timers: Record<string, { expiresAt: number; durationMs: number }> = {};
		for (const pu of powerups) {
			if (pu.remainingMs <= 0) continue;
			const meta = POWERUP_META[pu.kind];
			const appliesToMe = meta.affectsSelf
				? pu.sourcePlayerId === gs.playerId
				: pu.sourcePlayerId !== gs.playerId;
			if (!appliesToMe) continue;
			timers[pu.kind] = { expiresAt: now + pu.remainingMs, durationMs: pu.durationMs };
		}
		effectTimers = timers;
	});

	$effect(() => {
		if (gs.myPrompt && promptInputEl) {
			promptInputEl.focus();
		}
	});

	$effect(() => {
		if (gs.powerUpToast) {
			clearTimeout(powerUpToastTimeout);
			powerUpToastTimeout = window.setTimeout(() => {
				gs.powerUpToast = null;
			}, 3000);
		}
	});

	$effect(() => {
		function update() {
			visualHeight = window.visualViewport?.height ?? window.innerHeight;
		}
		update();
		window.visualViewport?.addEventListener('resize', update);
		return () => window.visualViewport?.removeEventListener('resize', update);
	});

	function animate(): void {
		if (gs.room && arenaEl) {
			blobLayout = nextBlobLayout(
				gs.room.players,
				blobLayout,
				performance.now(),
				arenaEl.clientWidth,
				arenaEl.clientHeight
			);
		}
		if (timerDisplayMs != null && timerSyncedAt > 0) {
			const elapsed = performance.now() - timerSyncedAt;
			timerDisplayMs = Math.max(0, timerBaseMs - elapsed);
		}

		const now = performance.now();
		const expired = gs.pendingPowerUps.filter((pu) => pu.expiresAt <= now);
		if (expired.length > 0) {
			gs.pendingPowerUps = gs.pendingPowerUps.filter((pu) => pu.expiresAt > now);
		}
		const offsets: Record<number, number> = {};
		for (const pu of gs.pendingPowerUps) {
			const remaining = Math.max(0, pu.expiresAt - now);
			const fraction = remaining / pu.durationMs;
			offsets[pu.offerId] = RING_CIRCUMFERENCE * (1 - fraction);
		}
		powerupRingOffsets = offsets;

		const fracs: Record<string, number> = {};
		for (const [kind, timer] of Object.entries(effectTimers)) {
			fracs[kind] = Math.max(0, (timer.expiresAt - now) / timer.durationMs);
		}
		effectFractions = fracs;

		animationHandle = requestAnimationFrame(animate);
	}

	function circleSize(player: PlayerSnapshot): number {
		return Math.max(42, Math.min(220, player.size * 4));
	}

	function leaveRoom(): void {
		disconnect();
		goto(resolve('/'));
	}

	function copyRoomLink(): void {
		navigator.clipboard.writeText(window.location.href);
		clearTimeout(copyTimeout);
		copyConfirmed = true;
		copyTimeout = window.setTimeout(() => (copyConfirmed = false), 1500);
	}

	onMount(() => {
		setOnDisconnect(() => goto(resolve('/')));

		if (gs.phase !== 'ingame') {
			const code = page.params.code ?? '';
			const session = loadSession();
			const rejoinToken = loadRejoinToken(code);
			connect(session?.wsUrl ?? defaultWsUrl(), {
				roomCode: code,
				gameMode: session?.gameMode,
				rejoinToken: rejoinToken ?? undefined
			});
		}

		animationHandle = requestAnimationFrame(animate);
	});

	onDestroy(() => {
		if (browser) {
			cancelAnimationFrame(animationHandle);
		}
		setOnDisconnect(null);
	});
</script>

<main class="game" style:--vvh={visualHeight ? `${visualHeight}px` : null}>
	<div class="leave">
		<Button label="Leave" onclick={leaveRoom} />
	</div>
	<header>
		{#if gs.room && gs.room.matchRemainingMs == null && !gs.room.matchWinner}
			<div class="lobby">
				<RulesDialog />
				{#if gs.playerId === gs.room.hostPlayerId}
					<div class="lobby-start">
						<Button label="Start Match" onclick={startMatch} />
					</div>
				{:else}
					<div class="lobby-wait shizuru-regular">Waiting for host to start...</div>
				{/if}
			</div>
		{:else}
			{#if timerDisplayMs != null && !gs.room?.matchWinner}
				<div class="timer" style:color={myColor}>
					<strong>{formatTimer(timerDisplayMs)}</strong>
				</div>
			{/if}
			{#if gs.myPrompt}
				<div class="prompt" class:shizuru-regular={promptScrambled}>
					<strong>{gs.myPrompt}</strong>
				</div>
			{:else if !gs.room?.matchWinner}
				<div class="prompt">
					<div class="host lobby-wait shizuru-regular">Waiting for prompt...</div>
				</div>
			{/if}
			{#if gs.room?.matchWinner}
				<div class="game-over-container">
					<h1 class="shizuru-regular">Game Over</h1>
					<div class="rematch-container">
						<Button label="Rematch" onclick={rematch} />
					</div>
				</div>
			{:else}
				{#if otherPendingPowerUps.length > 0}
					<div class="other-offers">
						{#each otherPendingPowerUps as pu (pu.offerId)}
							<PowerUpBadge
								emoji={POWERUP_META[pu.kind].emoji}
								label="{pu.playerName} vying for {POWERUP_META[pu.kind].label}"
								fraction={1 - (powerupRingOffsets[pu.offerId] ?? 0) / RING_CIRCUMFERENCE}
								barColor={pu.playerColor}
								labelColor={pu.playerColor}
								variant="offer"
							/>
						{/each}
					</div>
				{/if}
				{#if gs.powerUpToast}
					{@const meta = POWERUP_META[gs.powerUpToast]}
					{#key gs.powerUpToast}
						<div class="powerup-toast">
							{meta.emoji}
							{meta.label}
						</div>
					{/key}
				{/if}
				<div class="input-row">
					{#if myPendingPowerUps.length > 0}
						<div class="powerup-tray">
							{#each myPendingPowerUps as pu (pu.offerId)}
								<div class="powerup-slot">
									<svg class="countdown-ring" viewBox="0 0 40 40">
										<circle class="ring-bg" r="17" cx="20" cy="20" />
										<circle
											class="ring-fg"
											r="17"
											cx="20"
											cy="20"
											stroke-dasharray={RING_CIRCUMFERENCE}
											stroke-dashoffset={powerupRingOffsets[pu.offerId] ?? 0}
											style:stroke={myColor}
										/>
									</svg>
									<span class="powerup-emoji">{POWERUP_META[pu.kind].emoji}</span>
								</div>
							{/each}
						</div>
					{/if}
					{#if gs.myPrompt}
						<div class="input-container" class:disabled={inputDisabled}>
							<TextInput
								bind:el={promptInputEl}
								value={gs.promptInput}
								oninput={(e) => handlePromptInput(e.currentTarget.value)}
								onkeydown={(e) => {
									if (e.key === 'Enter' && !inputDisabled) submitPrompt();
								}}
								placeholder={gs.inputPlaceholder || 'Type your answer; press return.'}
								inputmode={gs.inputMode}
								enterkeyhint="go"
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
								disabled={inputDisabled}
								inlineButtonLabel="Go"
								inlineButtonOnclick={() => submitPrompt()}
							/>
						</div>
						{#if myActiveEffects.length > 0}
							<div class="active-effects">
								{#each myActiveEffects as effect (effect.kind)}
									<PowerUpBadge
										emoji={effect.emoji}
										label={effect.label}
										fraction={effectFractions[effect.kind] ?? 1}
										barColor={effect.disablesInput ? '#1e40af' : '#92400e'}
										variant={effect.disablesInput ? 'debuff' : 'buff'}
									/>
								{/each}
							</div>
						{/if}
					{/if}
				</div>
			{/if}
			{#if gs.latestRoundSummary}
				<div class="result" style:color={gs.latestRoundSummaryColor || null}>
					{gs.latestRoundSummary}
				</div>
			{/if}
		{/if}
	</header>
	<div class="arena" bind:this={arenaEl}>
		{#if gs.room}
			{#each gs.room.players as player (player.id)}
				<div
					class="blob {player.id === gs.playerId ? 'me' : ''}"
					style={`--blob-color:${player.color}; width:${circleSize(player)}px; height:${circleSize(player)}px; left:${(blobLayout[player.id]?.x ?? 0) - circleSize(player) / 2}px; top:${(blobLayout[player.id]?.y ?? 0) - circleSize(player) / 2}px;`}
				>
					<div class="name">{player.name}</div>
					<div class="powerup-emojis">{playerPowerUpEmojis(player.id)}</div>
					<div class="size">{player.size.toFixed(1)}</div>
					<div class="progress">{player.progress}</div>
				</div>
			{/each}
		{/if}
	</div>
	{#if gs.room?.roomCode}
		<div class="room">
			{#if copyConfirmed}
				<span class="copy-toast"><strong>LINK COPIED</strong></span>
			{/if}
			<input
				type="button"
				class="shizuru-regular"
				value={gs.room.roomCode}
				onclick={copyRoomLink}
			/>
		</div>
	{/if}
	{#if debugMode}
		<aside class="debug">
			<Button
				label={debugOpen ? 'Hide' : 'Stats for nerds'}
				onclick={() => (debugOpen = !debugOpen)}
			/>
			{#if debugOpen}
				<dl>
					<dt>game</dt>
					<dd>{gs.gameKey || 'unknown'}</dd>
					<dt>room</dt>
					<dd>{gs.room?.roomCode ?? '-'}</dd>
					<dt>socket</dt>
					<dd>{socketStateLabel()}</dd>
					<dt>inbound</dt>
					<dd>{gs.inboundCount}</dd>
					<dt>outbound</dt>
					<dd>{gs.outboundCount}</dd>
					<dt>players</dt>
					<dd>{gs.room?.players.length ?? 0}</dd>
				</dl>
			{/if}
		</aside>
	{/if}
</main>

<style>
	main {
		min-height: 100vh;
	}

	.game {
		display: grid;
		grid-template-rows: auto 1fr;
	}

	header {
		padding: 0.75rem;
		display: grid;
		gap: 0.5rem;
		position: relative;
		z-index: 2;
	}

	.lobby {
		text-align: center;
		margin-top: 3rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.5rem;
	}

	.host {
		padding-top: 6rem;
	}

	.lobby-wait {
		font-size: 3rem;
		margin: 0 auto;
		max-width: 400px;
	}

	.timer {
		font-size: 3rem;
		text-align: center;
		margin-top: 3.5rem;
		font-variant-numeric: tabular-nums;
	}

	.prompt {
		font-size: 2rem;
		text-align: center;
		margin: 1rem 0 2rem 0;
	}

	.input-row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		margin: 0 auto;
		width: 100%;
		max-width: 480px;
	}

	.input-container {
		display: flex;
		position: relative;
		flex: 1;
		min-width: 0;
	}

	.input-container.disabled {
		opacity: 0.5;
		pointer-events: none;
	}

	.powerup-tray {
		--slot-size: 40px;
		display: grid;
		grid-template-columns: repeat(2, auto);
		gap: 0.35rem;
		flex-shrink: 0;
		max-height: var(--slot-size);
		overflow: visible;
	}

	.powerup-slot {
		position: relative;
		width: var(--slot-size);
		height: var(--slot-size);
		display: grid;
		place-items: center;
	}

	.countdown-ring {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.ring-bg {
		fill: none;
		stroke: #e5e7eb;
		stroke-width: 3;
	}

	.ring-fg {
		fill: none;
		stroke: currentColor;
		stroke-width: 3;
		stroke-linecap: round;
		transform: rotate(-90deg);
		transform-origin: center;
	}

	.powerup-emoji {
		font-size: 1.2rem;
		line-height: 1;
		z-index: 1;
	}

	.other-offers {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		margin: 0 auto;
		max-width: 480px;
		width: 100%;
	}

	.powerup-toast {
		text-align: center;
		font-size: 1rem;
		font-weight: 700;
		padding: 0.35rem 0.75rem;
		border-radius: 0.5rem;
		background: #d1fae5;
		color: #065f46;
		animation: fade-in-out 3s ease forwards;
		margin: 0 auto 0.4rem;
	}

	.active-effects {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		flex-shrink: 0;
	}

	.game-over-container {
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}

	.game-over-container h1 {
		font-size: 5rem;
		margin-bottom: 1rem;
	}

	.result {
		font-size: 0.9rem;
		text-align: center;
		margin-top: 0.25rem;
	}

	.arena {
		position: relative;
		overflow: hidden;
		min-height: 62vh;
	}

	.blob {
		position: absolute;
		background: var(--blob-color);
		border: 2px solid var(--blob-color);
		border-radius: 9999px;
		display: grid;
		place-content: center;
		gap: 0.2rem;
		text-align: center;
		padding: 0.5rem;
		box-sizing: border-box;
		text-wrap: nowrap;
		transition:
			width 180ms linear,
			height 180ms linear;
	}

	.blob.me {
		outline: 2px solid;
	}

	.name {
		font-size: 0.85rem;
		font-weight: 600;
	}

	.powerup-emojis {
		font-size: 0.85rem;
		font-weight: 600;
	}

	.size,
	.progress {
		font-size: 0.75rem;
	}

	.leave {
		position: fixed;
		top: 0.5rem;
		left: 0.5rem;
		right: 0.5rem;
		z-index: 3;
	}

	.rematch-container,
	.lobby-start {
		display: flex;
		justify-content: center;
		width: 100%;
		max-width: 25rem;
		padding: 1rem 0 2rem 0;
	}

	.lobby-start {
		margin: 0 auto;
	}

	.room {
		position: fixed;
		bottom: 2rem;
		left: 0.5rem;
		right: 0.5rem;
		z-index: 3;
		text-align: center;
	}

	.copy-toast {
		display: block;
		animation: fade-in-out 1.5s ease forwards;
	}

	@keyframes fade-in-out {
		0% {
			opacity: 0;
			translate: 0 4px;
		}
		15% {
			opacity: 1;
			translate: 0 0;
		}
		75% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}

	.room input[type='button'] {
		background-color: transparent;
		border: none;
		color: black;
		font-size: 3rem;
		cursor: pointer;
	}

	.debug {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		position: fixed;
		right: 0.5rem;
		bottom: 0.5rem;
		border-radius: 0.4rem;
		padding: 0.5rem;
		width: 240px;
		z-index: 3;
	}

	dl {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.15rem 0.4rem;
		margin: 0.45rem 0 0 0;
		font-size: 0.8rem;
	}

	dd {
		margin: 0;
		text-align: right;
	}

	@media (max-width: 768px) and (orientation: portrait) {
		main {
			min-height: 0;
			height: var(--vvh, 100vh);
			max-height: var(--vvh, 100vh);
			overflow: hidden;
		}

		.arena {
			min-height: 0;
		}
	}
</style>
