import { browser } from '$app/environment';
import {
	decodeServerMessage,
	type ClientMessage,
	type PowerUpKind,
	type RoomSnapshot,
	type ServerMessage
} from './protocol';
import { m } from '$lib/paraglide/messages';

export type ConnectionPhase = 'pregame' | 'connecting' | 'ingame';

const SESSION_KEY = 'edifio-connection';
const REJOIN_PREFIX = 'edifio-rejoin-';

type SessionData = {
	gameMode: string;
	wsUrl: string;
};

function saveSession(data: SessionData): void {
	if (browser) {
		sessionStorage.setItem(SESSION_KEY, JSON.stringify(data));
	}
}

export function loadSession(): SessionData | null {
	if (!browser) return null;
	try {
		const raw = sessionStorage.getItem(SESSION_KEY);
		return raw ? (JSON.parse(raw) as SessionData) : null;
	} catch {
		return null;
	}
}

function saveRejoinToken(roomCode: string, token: string): void {
	if (browser) {
		sessionStorage.setItem(REJOIN_PREFIX + roomCode, token);
	}
}

export function loadRejoinToken(roomCode: string): string | null {
	if (!browser) return null;
	return sessionStorage.getItem(REJOIN_PREFIX + roomCode);
}

export function defaultWsUrl(): string {
	if (!browser) return 'ws://localhost:4000/ws';
	const wsProtocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${wsProtocol}//${location.host}/ws`;
}

function normalizeRoomCode(value: string): string {
	return value.trim().toUpperCase();
}

export type PendingPowerUp = {
	offerId: number;
	playerId: number;
	kind: PowerUpKind;
	expiresAt: number;
	durationMs: number;
};

export const gs = $state({
	phase: 'pregame' as ConnectionPhase,
	playerId: null as number | null,
	room: null as RoomSnapshot | null,
	roomCode: '',
	gameKey: '',
	inputPlaceholder: '',
	inputMode: 'text' as 'text' | 'none' | 'search' | 'tel' | 'url' | 'email' | 'numeric' | 'decimal',
	promptInput: '',
	myPrompt: '',
	latestRoundSummary: '',
	latestRoundSummaryColor: '',
	errorMessage: '',
	inboundCount: 0,
	outboundCount: 0,
	socketState: 'idle',
	lastSocketDetail: '',
	pendingPowerUps: [] as PendingPowerUp[],
	powerUpToast: null as PowerUpKind | null
});

let socket: WebSocket | null = null;
let welcomeCallback: ((roomCode: string) => void) | null = null;
let disconnectCallback: (() => void) | null = null;

export function setOnWelcome(fn: ((roomCode: string) => void) | null): void {
	welcomeCallback = fn;
}

export function setOnDisconnect(fn: (() => void) | null): void {
	disconnectCallback = fn;
}

export function sendClientMessage(message: ClientMessage): void {
	if (!socket || socket.readyState !== WebSocket.OPEN) return;
	gs.outboundCount += 1;
	socket.send(JSON.stringify(message));
}

function handleServerMessage(message: ServerMessage): void {
	switch (message.type) {
		case 'welcome':
			gs.playerId = message.playerId;
			gs.gameKey = message.gameKey;
			gs.inputPlaceholder = message.inputPlaceholder;
			gs.inputMode = message.inputMode;
			gs.roomCode = message.roomCode;
			gs.phase = 'ingame';
			saveRejoinToken(message.roomCode, message.rejoinToken);
			welcomeCallback?.(message.roomCode);
			break;
		case 'roomState':
			gs.room = message.room;
			gs.gameKey = message.room.gameKey;
			gs.inputMode = message.room.inputMode as typeof gs.inputMode;
			gs.inputPlaceholder = message.room.inputPlaceholder;
			if (message.room.matchWinner) {
				gs.pendingPowerUps = [];
				gs.powerUpToast = null;
				gs.myPrompt = '';
				const winner = message.room.players.find((p) => p.id === message.room.matchWinner);
				gs.latestRoundSummary = m.match_winner({
					winner: winner?.name ?? m.player_fallback({ id: message.room.matchWinner })
				});
				gs.latestRoundSummaryColor = winner?.color ?? '';
			}
			break;
		case 'promptState':
			if (message.playerId === gs.playerId) {
				gs.myPrompt = message.prompt;
				gs.promptInput = '';
			}
			break;
		case 'raceProgress':
			if (!gs.room) break;
			gs.room = {
				...gs.room,
				players: gs.room.players.map((p) =>
					p.id === message.playerId ? { ...p, progress: message.text } : p
				)
			};
			break;
		case 'roundResult':
			if (!gs.room) break;
			{
				const winner = gs.room.players.find((p) => p.id === message.winnerPlayerId);
				gs.latestRoundSummary = m.round_win({
					winner: winner?.name ?? m.player_fallback({ id: message.winnerPlayerId }),
					amount: message.growthAwarded.toFixed(1)
				});
				gs.latestRoundSummaryColor = winner?.color ?? '';
			}
			break;
		case 'wrongAnswer':
			if (!gs.room) break;
			{
				const isMe = message.playerId === gs.playerId;
				if (isMe) {
					gs.latestRoundSummary = m.wrong_self({ amount: message.shrinkApplied.toFixed(1) });
					gs.latestRoundSummaryColor = '#e74c3c';
					gs.promptInput = '';
				} else {
					const player = gs.room.players.find((p) => p.id === message.playerId);
					gs.latestRoundSummary = m.wrong_other({
						player: player?.name ?? m.player_fallback({ id: message.playerId }),
						amount: message.shrinkApplied.toFixed(1)
					});
					gs.latestRoundSummaryColor = player?.color ?? '';
				}
			}
			break;
		case 'error':
			gs.errorMessage = message.message;
			if (gs.phase === 'connecting') {
				gs.phase = 'pregame';
				socket?.close();
			}
			break;
		case 'powerUpOffered':
			gs.pendingPowerUps = [
				...gs.pendingPowerUps,
				{
					offerId: message.offerId,
					playerId: message.playerId,
					kind: message.kind,
					expiresAt: performance.now() + message.expiresInMs,
					durationMs: message.expiresInMs
				}
			];
			break;
		case 'powerUpOfferExpired': {
			const idx = gs.pendingPowerUps.findIndex((pu) => pu.offerId === message.offerId);
			if (idx !== -1) {
				gs.pendingPowerUps = gs.pendingPowerUps.toSpliced(idx, 1);
			}
			break;
		}
		case 'powerUpActivated': {
			const idx = gs.pendingPowerUps.findIndex((pu) => pu.offerId === message.offerId);
			if (idx !== -1) {
				gs.pendingPowerUps = gs.pendingPowerUps.toSpliced(idx, 1);
			}
			if (message.playerId === gs.playerId) {
				gs.powerUpToast = message.kind;
			}
			break;
		}
		case 'powerUpEffectEnded':
			break;
	}
}

export function connect(
	wsUrl: string,
	opts?: {
		roomCode?: string;
		gameMode?: string;
		matchDurationSecs?: number;
		gameOptions?: Record<string, string>;
		rejoinToken?: string;
	}
): void {
	if (gs.phase === 'connecting') return;

	gs.errorMessage = '';
	gs.latestRoundSummary = '';
	gs.latestRoundSummaryColor = '';
	gs.phase = 'connecting';
	gs.socketState = 'connecting';
	gs.lastSocketDetail = '';
	socket?.close();

	saveSession({
		gameMode: opts?.gameMode ?? '',
		wsUrl
	});

	try {
		socket = new WebSocket(wsUrl);
	} catch {
		gs.errorMessage = m.err_invalid_ws_url({ url: wsUrl });
		gs.phase = 'pregame';
		return;
	}

	socket.onopen = () => {
		gs.socketState = 'open';
		if (opts?.rejoinToken) {
			sendClientMessage({ type: 'rejoinRoom', rejoinToken: opts.rejoinToken });
		} else {
			const hasOptions = opts?.gameOptions && Object.keys(opts.gameOptions).length > 0;
			sendClientMessage({
				type: 'joinOrCreateRoom',
				roomCode: opts?.roomCode ? normalizeRoomCode(opts.roomCode) : undefined,
				gameMode: opts?.gameMode || undefined,
				matchDurationSecs: opts?.matchDurationSecs,
				gameOptions: hasOptions ? opts!.gameOptions : undefined
			});
		}
	};

	socket.onmessage = (event: MessageEvent) => {
		const decoded = decodeServerMessage(String(event.data));
		if (!decoded) return;
		gs.inboundCount += 1;
		handleServerMessage(decoded);
	};

	socket.onerror = () => {
		gs.errorMessage = m.err_ws_generic();
		gs.lastSocketDetail = 'socket error event fired';
	};

	socket.onclose = (event: CloseEvent) => {
		gs.socketState = 'closed';
		gs.lastSocketDetail = `closed code=${event.code} reason=${event.reason || '(none)'}`;
		const wasActive = gs.phase !== 'pregame';
		if (wasActive) {
			gs.errorMessage = m.err_disconnected();
		}
		gs.phase = 'pregame';
		gs.room = null;
		gs.myPrompt = '';
		if (wasActive) {
			disconnectCallback?.();
		}
	};
}

export function disconnect(): void {
	socket?.close();
	socket = null;
	gs.phase = 'pregame';
	gs.room = null;
	gs.myPrompt = '';
}

export function socketStateLabel(): string {
	if (!socket) return 'closed';
	if (socket.readyState === WebSocket.OPEN) return 'open';
	if (socket.readyState === WebSocket.CONNECTING) return 'connecting';
	if (socket.readyState === WebSocket.CLOSING) return 'closing';
	return 'closed';
}

export function handlePromptInput(value: string): void {
	gs.promptInput = value;
	sendClientMessage({ type: 'inputUpdate', text: value });
}

export function submitPrompt(): void {
	sendClientMessage({ type: 'submitAttempt', text: gs.promptInput });
}

export function startMatch(): void {
	sendClientMessage({ type: 'startMatch' });
}

export function rematch(): void {
	gs.latestRoundSummary = '';
	gs.latestRoundSummaryColor = '';
	gs.promptInput = '';
	gs.myPrompt = '';
	gs.pendingPowerUps = [];
	sendClientMessage({ type: 'rematch' });
}

export function updateRoomSettings(args: {
	gameMode?: string;
	matchDurationSecs?: number;
	gameOptions?: Record<string, string>;
}): void {
	sendClientMessage({ type: 'updateRoomSettings', ...args });
}
