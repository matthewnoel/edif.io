// Mock data for the dev playground (see /dev). Reuses the real protocol types and
// the real blob-layout math from sim.ts so previews match production rendering.
import type {
	ActivePowerUpSnapshot,
	GameModeInfo,
	PlayerSnapshot,
	PowerUpKind,
	RoomSnapshot
} from '$lib/game/protocol';
import { blobRadius, nextBlobLayout, type BlobLayout } from '$lib/game/sim';
import type { BlobView } from '$lib/components/views/GameRoomView.svelte';

export const PALETTE = [
	'#38bdf8',
	'#a78bfa',
	'#34d399',
	'#f472b6',
	'#fbbf24',
	'#fb7185',
	'#22d3ee'
];

const NAMES = ['Ada', 'Babbage', 'Curie', 'Dijkstra', 'Euler', 'Fermat', 'Gauss'];

export function player(over: Partial<PlayerSnapshot> = {}): PlayerSnapshot {
	const id = over.id ?? 1;
	return {
		id,
		name: NAMES[(id - 1) % NAMES.length],
		size: 10,
		color: PALETTE[(id - 1) % PALETTE.length],
		connected: true,
		progress: '',
		...over
	};
}

export function players(n: number): PlayerSnapshot[] {
	return Array.from({ length: n }, (_, i) =>
		player({ id: i + 1, size: 8 + (n - i) * 6 + (i % 2 ? 3 : 0) })
	);
}

export function activePowerUp(
	kind: PowerUpKind,
	over: Partial<ActivePowerUpSnapshot> = {}
): ActivePowerUpSnapshot {
	return { kind, sourcePlayerId: 1, remainingMs: 6000, durationMs: 12000, ...over };
}

function baseRoom(over: Partial<RoomSnapshot> = {}): RoomSnapshot {
	return {
		roomCode: 'WXYZ',
		players: players(4),
		matchWinner: null,
		matchRemainingMs: null,
		hostPlayerId: 1,
		activePowerups: [],
		gameKey: 'arithmetic',
		gameOptions: {},
		matchDurationSecs: 60,
		inputMode: 'decimal',
		inputPlaceholder: 'Enter the solution; press return.',
		...over
	};
}

export function roomLobby(over: Partial<RoomSnapshot> = {}): RoomSnapshot {
	return baseRoom({ matchRemainingMs: null, matchWinner: null, ...over });
}

export function roomMidMatch(over: Partial<RoomSnapshot> = {}): RoomSnapshot {
	return baseRoom({
		matchRemainingMs: 42000,
		matchWinner: null,
		players: players(4).map((p, i) => ({ ...p, progress: `${4 + i}/20` })),
		...over
	});
}

export function roomGameOver(over: Partial<RoomSnapshot> = {}): RoomSnapshot {
	return baseRoom({
		matchRemainingMs: 0,
		matchWinner: 1,
		players: players(4).map((p, i) => ({ ...p, progress: `${18 + i}/20` })),
		...over
	});
}

/**
 * Build static BlobView[] for GameRoomView previews. Runs the real layout a few
 * dozen times at a fixed elapsed time so the lerped positions settle, giving a
 * deterministic snapshot that matches the live mid-frame look.
 */
export function staticBlobs(
	ps: PlayerSnapshot[],
	myId: number,
	emojisFor: Record<number, string> = {},
	w = 390,
	h = 480
): BlobView[] {
	let layout: BlobLayout = {};
	for (let i = 0; i < 80; i += 1) {
		layout = nextBlobLayout(ps, layout, 2000, w, h);
	}
	return ps.map((p) => ({
		id: p.id,
		name: p.name,
		color: p.color,
		progress: p.progress,
		size: p.size,
		isMe: p.id === myId,
		emojis: emojisFor[p.id] ?? '',
		x: layout[p.id]?.x ?? w / 2,
		y: layout[p.id]?.y ?? h / 2,
		diameter: blobRadius(p, w, h) * 2
	}));
}

// Mirrors GET /api/game-modes (see core/src/server.rs + adapters/*). Keeps the
// GameSetupForm stories exercising select / range / toggle + visibleWhen + the
// firstTerm*/secondTerm* range-pair clamping in GameSetupForm.
export const gameModes: GameModeInfo[] = [
	{
		key: 'arithmetic',
		label: 'Arithmetic',
		options: [
			{
				key: 'operation',
				label: 'Operation',
				type: 'select',
				default: 'addition',
				choices: [
					{ value: 'addition', label: 'Addition' },
					{ value: 'subtraction', label: 'Subtraction' },
					{ value: 'multiplication', label: 'Multiplication' },
					{ value: 'division', label: 'Division' }
				]
			},
			rangeField('firstTermMinimumDigits', 'Minimum Digits in First Term'),
			rangeField('firstTermMaximumDigits', 'Maximum Digits in First Term'),
			rangeField('secondTermMinimumDigits', 'Minimum Digits in Second Term'),
			rangeField('secondTermMaximumDigits', 'Maximum Digits in Second Term'),
			{
				key: 'allowNegativeAnswers',
				label: 'Allow Negative Answers',
				type: 'toggle',
				default: false,
				visibleWhen: { key: 'operation', value: 'subtraction' }
			}
		]
	},
	{ key: 'keyboarding', label: 'Keyboarding', options: [] },
	{
		key: 'state-abbreviations',
		label: 'US State Abbreviations',
		options: [
			{
				key: 'direction',
				label: 'Prompt Direction',
				type: 'select',
				default: 'nameToAbbr',
				choices: [
					{ value: 'nameToAbbr', label: 'State Name → Abbreviation' },
					{ value: 'abbrToName', label: 'Abbreviation → State Name' },
					{ value: 'both', label: 'Both Directions' }
				]
			}
		]
	}
];

function rangeField(key: string, label: string): GameModeInfo['options'][number] {
	return { key, label, type: 'range', min: 1, max: 6, step: 1, default: 1 };
}

export function defaultOptionValues(mode: GameModeInfo): Record<string, string> {
	const out: Record<string, string> = {};
	for (const opt of mode.options) out[opt.key] = String(opt.default);
	return out;
}
