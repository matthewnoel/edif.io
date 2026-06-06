<script module lang="ts">
	export type BlobView = {
		id: number;
		name: string;
		color: string;
		progress: string;
		size: number;
		isMe: boolean;
		emojis: string;
		x: number;
		y: number;
		diameter: number;
	};

	export type ActiveEffectView = {
		kind: string;
		emoji: string;
		label: string;
		fraction: number;
		disablesInput: boolean;
	};

	export type MyOfferView = {
		offerId: number;
		emoji: string;
		ringOffset: number;
	};

	export type OtherOfferView = {
		offerId: number;
		emoji: string;
		label: string;
		fraction: number;
		color: string;
	};

	export type GameRoomInputMode =
		| 'text'
		| 'none'
		| 'search'
		| 'tel'
		| 'url'
		| 'email'
		| 'numeric'
		| 'decimal';

	export type DebugInfo = {
		gameKey: string;
		roomCode: string;
		socket: string;
		inbound: number;
		outbound: number;
		players: number;
	};
</script>

<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import PowerUpBadge from '$lib/components/PowerUpBadge.svelte';
	import RulesDialog from '$lib/components/RulesDialog.svelte';
	import TextInput from '$lib/components/TextInput.svelte';
	import { LEAVE_ICON, SETTINGS_ICON } from '$lib/constants';

	const RING_CIRCUMFERENCE = 106.81;

	interface Props {
		// phase
		inLobby: boolean;
		isHost: boolean;
		gameOver: boolean;

		// header
		timerLabel: string | null;
		myColor: string | null;
		myPrompt: string;
		promptScrambled: boolean;
		showWaitingForPrompt: boolean;
		roundSummary: string;
		roundSummaryColor: string;

		// input
		promptInput: string;
		inputPlaceholder: string;
		inputMode: GameRoomInputMode;
		inputDisabled: boolean;
		promptInputEl?: HTMLInputElement | null;

		// power-ups
		activeEffects: ActiveEffectView[];
		myPendingPowerUps: MyOfferView[];
		otherPendingPowerUps: OtherOfferView[];
		powerUpToast: { emoji: string; label: string } | null;

		// arena
		blobs: BlobView[];
		arenaEl?: HTMLDivElement | null;

		// chrome
		roomCode: string | null;
		copyConfirmed: boolean;
		visualHeight: number;
		debug: boolean;
		debugOpen?: boolean;
		debugInfo?: DebugInfo | null;

		// callbacks
		onleave: () => void;
		onedit: () => void;
		onstart: () => void;
		onrematch: () => void;
		onrefresh: () => void;
		oncopy: () => void;
		oninput: (value: string) => void;
		onsubmit: () => void;
	}

	let {
		inLobby,
		isHost,
		gameOver,
		timerLabel,
		myColor,
		myPrompt,
		promptScrambled,
		showWaitingForPrompt,
		roundSummary,
		roundSummaryColor,
		promptInput,
		inputPlaceholder,
		inputMode,
		inputDisabled,
		promptInputEl = $bindable(null),
		activeEffects,
		myPendingPowerUps,
		otherPendingPowerUps,
		powerUpToast,
		blobs,
		arenaEl = $bindable(null),
		roomCode,
		copyConfirmed,
		visualHeight,
		debug,
		debugOpen = $bindable(false),
		debugInfo = null,
		onleave,
		onedit,
		onstart,
		onrematch,
		onrefresh,
		oncopy,
		oninput,
		onsubmit
	}: Props = $props();
</script>

<main class="game" style:--vvh={visualHeight ? `${visualHeight}px` : null}>
	{#if inLobby}
		<RulesDialog />
	{/if}
	<div class="leave">
		<Button label={LEAVE_ICON} onclick={onleave} />
		{#if isHost}
			<Button label={SETTINGS_ICON} onclick={onedit} />
		{/if}
	</div>
	<header>
		{#if inLobby}
			<div class="lobby">
				{#if isHost}
					<div class="wide-button">
						<Button label="Start Match" onclick={onstart} />
					</div>
				{:else}
					<div class="lobby-wait shizuru-regular">Waiting for host to start...</div>
				{/if}
			</div>
		{:else}
			{#if timerLabel != null}
				<div class="timer" style:color={myColor}>
					<strong>{timerLabel}</strong>
				</div>
			{/if}
			{#if myPrompt}
				<div class="prompt" class:shizuru-regular={promptScrambled}>
					<strong>{myPrompt}</strong>
				</div>
			{:else if showWaitingForPrompt}
				<div class="prompt">
					<div class="host lobby-wait shizuru-regular">Waiting for prompt...</div>
				</div>
				<div class="wide-button">
					<Button label="Refresh" onclick={onrefresh} />
				</div>
			{/if}
			{#if gameOver}
				<div class="game-over-container">
					<h1 class="shizuru-regular">Game Over</h1>
					<div class="wide-button">
						<Button label="Rematch" onclick={onrematch} />
					</div>
				</div>
			{:else}
				{#if otherPendingPowerUps.length > 0}
					<div class="other-offers">
						{#each otherPendingPowerUps as pu (pu.offerId)}
							<PowerUpBadge
								emoji={pu.emoji}
								label={pu.label}
								fraction={pu.fraction}
								barColor={pu.color}
								labelColor={pu.color}
								variant="offer"
							/>
						{/each}
					</div>
				{/if}
				{#if powerUpToast}
					{#key powerUpToast.label}
						<div class="powerup-toast">
							{powerUpToast.emoji}
							{powerUpToast.label}
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
											stroke-dashoffset={pu.ringOffset}
											style:stroke={myColor}
										/>
									</svg>
									<span class="powerup-emoji">{pu.emoji}</span>
								</div>
							{/each}
						</div>
					{/if}
					{#if myPrompt}
						<div class="input-container" class:disabled={inputDisabled}>
							<TextInput
								bind:el={promptInputEl}
								value={promptInput}
								oninput={(e) => oninput(e.currentTarget.value)}
								onkeydown={(e) => {
									if (e.key === 'Enter' && !inputDisabled) onsubmit();
								}}
								placeholder={inputPlaceholder || 'Type your answer; press return.'}
								inputmode={inputMode}
								enterkeyhint="go"
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
								disabled={inputDisabled}
								inlineButtonLabel="Go"
								inlineButtonOnclick={() => onsubmit()}
							/>
						</div>
						{#if activeEffects.length > 0}
							<div class="active-effects">
								{#each activeEffects as effect (effect.kind)}
									<PowerUpBadge
										emoji={effect.emoji}
										label={effect.label}
										fraction={effect.fraction}
										barColor={effect.disablesInput ? '#1e40af' : '#92400e'}
										variant={effect.disablesInput ? 'debuff' : 'buff'}
									/>
								{/each}
							</div>
						{/if}
					{/if}
				</div>
			{/if}
			{#if roundSummary}
				<div class="result" style:color={roundSummaryColor || null}>
					{roundSummary}
				</div>
			{/if}
		{/if}
	</header>
	<div class="arena" bind:this={arenaEl}>
		{#each blobs as blob (blob.id)}
			<div
				class="blob {blob.isMe ? 'me' : ''}"
				style={`--blob-color:${blob.color}; width:${blob.diameter}px; height:${blob.diameter}px; left:${blob.x - blob.diameter / 2}px; top:${blob.y - blob.diameter / 2}px;`}
			>
				{#if blob.isMe}
					<div class="you-tag">YOU</div>
				{/if}
				<div class="name">{blob.name}</div>
				<div class="powerup-emojis">{blob.emojis}</div>
				<div class="size">{blob.size.toFixed(1)}</div>
				<div class="progress">{blob.progress}</div>
			</div>
		{/each}
	</div>
	{#if roomCode}
		<div class="room">
			{#if copyConfirmed}
				<span class="copy-toast"><strong>LINK COPIED</strong></span>
			{/if}
			<input type="button" class="shizuru-regular" value={roomCode} onclick={oncopy} />
		</div>
	{/if}
	{#if debug}
		<aside class="debug">
			<Button
				label={debugOpen ? 'Hide' : 'Stats for nerds'}
				onclick={() => (debugOpen = !debugOpen)}
			/>
			{#if debugOpen && debugInfo}
				<dl>
					<dt>game</dt>
					<dd>{debugInfo.gameKey || 'unknown'}</dd>
					<dt>room</dt>
					<dd>{debugInfo.roomCode || '-'}</dd>
					<dt>socket</dt>
					<dd>{debugInfo.socket}</dd>
					<dt>inbound</dt>
					<dd>{debugInfo.inbound}</dd>
					<dt>outbound</dt>
					<dd>{debugInfo.outbound}</dd>
					<dt>players</dt>
					<dd>{debugInfo.players}</dd>
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
		margin-top: 6rem;
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
		color: #fff;
		text-shadow:
			0 0 3px rgba(0, 0, 0, 0.9),
			0 1px 2px rgba(0, 0, 0, 0.9);
		transition:
			width 180ms linear,
			height 180ms linear;
	}

	.blob.me {
		box-shadow:
			inset 0 0 0 3px #fff,
			0 0 0 3px #111,
			0 0 22px 6px color-mix(in srgb, var(--blob-color) 75%, transparent);
		z-index: 1;
		animation: me-pulse 1.8s ease-in-out infinite;
	}

	@keyframes me-pulse {
		0%,
		100% {
			box-shadow:
				inset 0 0 0 3px #fff,
				0 0 0 3px #111,
				0 0 16px 3px color-mix(in srgb, var(--blob-color) 60%, transparent);
		}
		50% {
			box-shadow:
				inset 0 0 0 3px #fff,
				0 0 0 3px #111,
				0 0 28px 10px color-mix(in srgb, var(--blob-color) 90%, transparent);
		}
	}

	.you-tag {
		justify-self: center;
		background: #fff;
		color: #111;
		font-size: 0.7rem;
		font-weight: 900;
		letter-spacing: 0.12em;
		padding: 0.12rem 0.5rem;
		border-radius: 9999px;
		white-space: nowrap;
		pointer-events: none;
		text-shadow: none;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
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
		z-index: 3;
		display: flex;
		gap: 0.5rem;
	}

	.leave :global(.btn) {
		flex: 0 0 auto;
		padding: 0.4rem 0.7rem;
		font-size: 1.1rem;
		line-height: 1;
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
