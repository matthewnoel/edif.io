export type ErrorCode =
	| 'roomNotFound'
	| 'invalidGameMode'
	| 'invalidMessageFormat'
	| 'invalidRejoinToken'
	| 'roomExpired'
	| 'playerNotInRoom';

export type PowerUpKind =
	| 'freezeAllCompetitors'
	| 'doublePoints'
	| 'scrambleFont'
	| 'scoreSteal'
	| 'ongoingScoreSteal';

export type ActivePowerUpSnapshot = {
	kind: PowerUpKind;
	sourcePlayerId: number;
	remainingMs: number;
	durationMs: number;
};

export type PlayerSnapshot = {
	id: number;
	name: string;
	size: number;
	color: string;
	connected: boolean;
	progress: string;
};

export type RoomSnapshot = {
	roomCode: string;
	players: PlayerSnapshot[];
	prompt: string;
	roundId: number;
	matchWinner: number | null;
	matchRemainingMs: number | null;
	hostPlayerId: number;
	activePowerups: ActivePowerUpSnapshot[];
};

export type SelectChoice = {
	value: string;
	label: string;
};

export type OptionField = {
	key: string;
	label: string;
	type: 'select';
	choices: SelectChoice[];
	default: string;
};

export type GameModeInfo = {
	key: string;
	label: string;
	options: OptionField[];
};

export type ClientMessage =
	| {
			type: 'joinOrCreateRoom';
			playerName?: string;
			roomCode?: string;
			gameMode?: string;
			matchDurationSecs?: number;
			gameOptions?: Record<string, string>;
	  }
	| { type: 'rejoinRoom'; rejoinToken: string }
	| { type: 'inputUpdate'; text: string }
	| { type: 'submitAttempt'; text: string }
	| { type: 'startMatch' }
	| { type: 'rematch' };

export type ServerMessage =
	| {
			type: 'welcome';
			playerId: number;
			roomCode: string;
			gameKey: string;
			inputPlaceholder: string;
			rejoinToken: string;
	  }
	| { type: 'roomState'; room: RoomSnapshot }
	| { type: 'promptState'; roomCode: string; roundId: number; prompt: string }
	| { type: 'raceProgress'; roomCode: string; playerId: number; text: string }
	| {
			type: 'roundResult';
			roomCode: string;
			roundId: number;
			winnerPlayerId: number;
			growthAwarded: number;
	  }
	| { type: 'wrongAnswer'; roomCode: string; playerId: number; shrinkApplied: number }
	| { type: 'error'; message: string; code?: ErrorCode }
	| {
			type: 'powerUpOffered';
			offerId: number;
			playerId: number;
			kind: PowerUpKind;
			expiresInMs: number;
	  }
	| {
			type: 'powerUpActivated';
			offerId: number;
			playerId: number;
			kind: PowerUpKind;
			durationMs: number;
	  }
	| { type: 'powerUpOfferExpired'; offerId: number; playerId: number; kind: PowerUpKind }
	| { type: 'powerUpEffectEnded'; playerId: number; kind: PowerUpKind };

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null;
}

const VALID_ERROR_CODES: ErrorCode[] = [
	'roomNotFound',
	'invalidGameMode',
	'invalidMessageFormat',
	'invalidRejoinToken',
	'roomExpired',
	'playerNotInRoom'
];

function isErrorCode(value: unknown): value is ErrorCode {
	return typeof value === 'string' && VALID_ERROR_CODES.includes(value as ErrorCode);
}

const VALID_POWERUP_KINDS: PowerUpKind[] = [
	'freezeAllCompetitors',
	'doublePoints',
	'scrambleFont',
	'scoreSteal',
	'ongoingScoreSteal'
];

function isPowerUpKind(value: unknown): value is PowerUpKind {
	return typeof value === 'string' && VALID_POWERUP_KINDS.includes(value as PowerUpKind);
}

function isActivePowerUpSnapshot(value: unknown): value is ActivePowerUpSnapshot {
	if (!isObject(value)) return false;
	return (
		isPowerUpKind(value.kind) &&
		typeof value.sourcePlayerId === 'number' &&
		typeof value.remainingMs === 'number' &&
		typeof value.durationMs === 'number'
	);
}

function isPlayerSnapshot(value: unknown): value is PlayerSnapshot {
	if (!isObject(value)) return false;
	return (
		typeof value.id === 'number' &&
		typeof value.name === 'string' &&
		typeof value.size === 'number' &&
		typeof value.color === 'string' &&
		typeof value.connected === 'boolean' &&
		typeof value.progress === 'string'
	);
}

function isRoomSnapshot(value: unknown): value is RoomSnapshot {
	if (!isObject(value) || !Array.isArray(value.players)) return false;
	return (
		typeof value.roomCode === 'string' &&
		typeof value.prompt === 'string' &&
		typeof value.roundId === 'number' &&
		(value.matchWinner === null || typeof value.matchWinner === 'number') &&
		(value.matchRemainingMs === null || typeof value.matchRemainingMs === 'number') &&
		typeof value.hostPlayerId === 'number' &&
		value.players.every(isPlayerSnapshot) &&
		Array.isArray(value.activePowerups) &&
		value.activePowerups.every(isActivePowerUpSnapshot)
	);
}

function isServerMessage(value: unknown): value is ServerMessage {
	if (!isObject(value) || typeof value.type !== 'string') {
		return false;
	}
	switch (value.type) {
		case 'welcome':
			return (
				typeof value.playerId === 'number' &&
				typeof value.roomCode === 'string' &&
				typeof value.gameKey === 'string' &&
				typeof value.inputPlaceholder === 'string' &&
				typeof value.rejoinToken === 'string'
			);
		case 'roomState':
			return isRoomSnapshot(value.room);
		case 'promptState':
			return (
				typeof value.roomCode === 'string' &&
				typeof value.roundId === 'number' &&
				typeof value.prompt === 'string'
			);
		case 'raceProgress':
			return (
				typeof value.roomCode === 'string' &&
				typeof value.playerId === 'number' &&
				typeof value.text === 'string'
			);
		case 'roundResult':
			return (
				typeof value.roomCode === 'string' &&
				typeof value.roundId === 'number' &&
				typeof value.winnerPlayerId === 'number' &&
				typeof value.growthAwarded === 'number'
			);
		case 'wrongAnswer':
			return (
				typeof value.roomCode === 'string' &&
				typeof value.playerId === 'number' &&
				typeof value.shrinkApplied === 'number'
			);
		case 'error':
			return (
				typeof value.message === 'string' && (value.code === undefined || isErrorCode(value.code))
			);
		case 'powerUpOffered':
			return (
				typeof value.offerId === 'number' &&
				typeof value.playerId === 'number' &&
				isPowerUpKind(value.kind) &&
				typeof value.expiresInMs === 'number'
			);
		case 'powerUpActivated':
			return (
				typeof value.offerId === 'number' &&
				typeof value.playerId === 'number' &&
				isPowerUpKind(value.kind) &&
				typeof value.durationMs === 'number'
			);
		case 'powerUpOfferExpired':
			return (
				typeof value.offerId === 'number' &&
				typeof value.playerId === 'number' &&
				isPowerUpKind(value.kind)
			);
		case 'powerUpEffectEnded':
			return typeof value.playerId === 'number' && isPowerUpKind(value.kind);
		default:
			return false;
	}
}

export function decodeServerMessage(raw: string): ServerMessage | null {
	try {
		const parsed: unknown = JSON.parse(raw);
		return isServerMessage(parsed) ? parsed : null;
	} catch {
		return null;
	}
}
