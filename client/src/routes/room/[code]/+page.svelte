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
	import { nextBlobLayout, blobRadius, type BlobLayout } from '$lib/game/sim';
	import type { PlayerSnapshot, PowerUpKind } from '$lib/game/protocol';
	import { debugMode } from '$lib/debug';
	import GameRoomView from '$lib/components/views/GameRoomView.svelte';
	import { m } from '$lib/paraglide/messages';
	import { inputPlaceholder as localizedPlaceholder } from '$lib/i18n/chrome';

	type PowerUpMeta = {
		emoji: string;
		affectsSelf: boolean;
		disablesInput: boolean;
	};

	const POWERUP_META: Record<PowerUpKind, PowerUpMeta> = {
		doublePoints: {
			emoji: '\u{1F4AA}',
			affectsSelf: true,
			disablesInput: false
		},
		scrambleFont: {
			emoji: '\u{1F92A}',
			affectsSelf: false,
			disablesInput: false
		},
		scoreSteal: {
			emoji: '\u{1F422}',
			affectsSelf: true,
			disablesInput: false
		},
		ongoingScoreSteal: {
			emoji: '\u{1F355}',
			affectsSelf: true,
			disablesInput: false
		}
	};

	// Power-up display names are viewer-language UI, keyed by PowerUpKind.
	const POWERUP_LABEL: Record<PowerUpKind, () => string> = {
		doublePoints: m.powerup_doublePoints,
		scrambleFont: m.powerup_scrambleFont,
		scoreSteal: m.powerup_scoreSteal,
		ongoingScoreSteal: m.powerup_ongoingScoreSteal
	};

	function powerupLabel(kind: PowerUpKind): string {
		return POWERUP_LABEL[kind]();
	}

	const RING_CIRCUMFERENCE = 106.81;

	let arenaEl: HTMLDivElement | null = $state(null);
	let blobLayout: BlobLayout = $state({});
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
		const width = arenaEl?.clientWidth ?? 0;
		const height = arenaEl?.clientHeight ?? 0;
		if (width === 0 || height === 0) {
			return Math.max(42, Math.min(220, player.size * 4));
		}
		return blobRadius(player, width, height) * 2;
	}

	function leaveRoom(): void {
		disconnect();
		goto(resolve('/'));
	}

	function editRoom(): void {
		goto(resolve(`/room/${page.params.code}/edit`));
	}

	function copyRoomLink(): void {
		navigator.clipboard.writeText(window.location.href);
		clearTimeout(copyTimeout);
		copyConfirmed = true;
		copyTimeout = window.setTimeout(() => (copyConfirmed = false), 1500);
	}

	// --- View-model derived from the live store + animation state ---

	let inLobby = $derived(!!gs.room && gs.room.matchRemainingMs == null && !gs.room.matchWinner);
	let isHost = $derived(!!gs.room && gs.playerId === gs.room.hostPlayerId);
	let gameOver = $derived(!!gs.room?.matchWinner);
	let timerLabel = $derived(
		timerDisplayMs != null && !gameOver ? formatTimer(timerDisplayMs) : null
	);
	let showWaitingForPrompt = $derived(!gs.myPrompt && !gameOver);

	let activeEffectsView = $derived(
		myActiveEffects.map((e) => ({
			kind: e.kind,
			emoji: e.emoji,
			label: powerupLabel(e.kind),
			fraction: effectFractions[e.kind] ?? 1,
			disablesInput: e.disablesInput
		}))
	);

	let myPendingView = $derived(
		myPendingPowerUps.map((pu) => ({
			offerId: pu.offerId,
			emoji: POWERUP_META[pu.kind].emoji,
			ringOffset: powerupRingOffsets[pu.offerId] ?? 0
		}))
	);

	let otherPendingView = $derived(
		otherPendingPowerUps.map((pu) => ({
			offerId: pu.offerId,
			emoji: POWERUP_META[pu.kind].emoji,
			label: m.powerup_vying({ player: pu.playerName, label: powerupLabel(pu.kind) }),
			fraction: 1 - (powerupRingOffsets[pu.offerId] ?? 0) / RING_CIRCUMFERENCE,
			color: pu.playerColor
		}))
	);

	let powerUpToastView = $derived(
		gs.powerUpToast
			? { emoji: POWERUP_META[gs.powerUpToast].emoji, label: powerupLabel(gs.powerUpToast) }
			: null
	);

	let blobs = $derived(
		(gs.room?.players ?? []).map((player) => ({
			id: player.id,
			name: player.name,
			color: player.color,
			progress: player.progress,
			size: player.size,
			isMe: player.id === gs.playerId,
			emojis: playerPowerUpEmojis(player.id),
			x: blobLayout[player.id]?.x ?? 0,
			y: blobLayout[player.id]?.y ?? 0,
			diameter: circleSize(player)
		}))
	);

	let debugInfo = $derived({
		gameKey: gs.gameKey,
		roomCode: gs.room?.roomCode ?? '',
		socket: socketStateLabel(),
		inbound: gs.inboundCount,
		outbound: gs.outboundCount,
		players: gs.room?.players.length ?? 0
	});

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

<GameRoomView
	{inLobby}
	{isHost}
	{gameOver}
	{timerLabel}
	{myColor}
	myPrompt={gs.myPrompt}
	{promptScrambled}
	{showWaitingForPrompt}
	roundSummary={gs.latestRoundSummary}
	roundSummaryColor={gs.latestRoundSummaryColor}
	promptInput={gs.promptInput}
	inputPlaceholder={localizedPlaceholder(gs.gameKey, gs.inputPlaceholder)}
	inputMode={gs.inputMode}
	{inputDisabled}
	bind:promptInputEl
	activeEffects={activeEffectsView}
	myPendingPowerUps={myPendingView}
	otherPendingPowerUps={otherPendingView}
	powerUpToast={powerUpToastView}
	{blobs}
	bind:arenaEl
	roomCode={gs.room?.roomCode ?? null}
	{copyConfirmed}
	{visualHeight}
	debug={debugMode}
	{debugInfo}
	onleave={leaveRoom}
	onedit={editRoom}
	onstart={startMatch}
	onrematch={rematch}
	onrefresh={() => window.location.reload()}
	oncopy={copyRoomLink}
	oninput={handlePromptInput}
	onsubmit={submitPrompt}
/>
