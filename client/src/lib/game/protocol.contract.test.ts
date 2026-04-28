/**
 * Cross-language wire-format contract tests.
 *
 * These fixtures mirror the snapshot tests in `core/src/protocol.rs`
 * (`wire_*_shape` and friends). Every JSON payload in this file must match,
 * byte-for-byte at the semantic level, what the Rust server emits.
 *
 * If a dependency bump (serde, serde_json, etc.) silently changes the shape
 * on the server side, the Rust snapshot tests fail. If it changes the shape
 * that the TypeScript validator accepts, these tests fail. Either way,
 * a drift between server and client is caught before reaching production.
 */
import { describe, expect, it } from 'vitest';
import { decodeServerMessage, type ServerMessage } from './protocol';

function decode(value: unknown): ServerMessage {
	const parsed = decodeServerMessage(JSON.stringify(value));
	if (parsed === null) {
		throw new Error(`decodeServerMessage rejected valid payload: ${JSON.stringify(value)}`);
	}
	return parsed;
}

describe('server → client wire contract', () => {
	it('accepts the exact welcome shape emitted by the server', () => {
		const msg = decode({
			type: 'welcome',
			playerId: 4,
			roomCode: 'ABCD',
			gameKey: 'keyboarding',
			inputPlaceholder: 'Type here...',
			inputMode: 'text',
			rejoinToken: 'abc123'
		});
		expect(msg.type).toBe('welcome');
		if (msg.type === 'welcome') {
			expect(msg.playerId).toBe(4);
			expect(msg.roomCode).toBe('ABCD');
			expect(msg.gameKey).toBe('keyboarding');
			expect(msg.rejoinToken).toBe('abc123');
		}
	});

	it('accepts the exact roomState shape emitted by the server', () => {
		const msg = decode({
			type: 'roomState',
			room: {
				roomCode: 'ABCD',
				players: [
					{
						id: 1,
						name: 'Alice',
						size: 14.5,
						color: '#38bdf8',
						connected: true,
						progress: 'he'
					}
				],
				matchWinner: null,
				matchRemainingMs: 45000,
				hostPlayerId: 1,
				activePowerups: [
					{
						kind: 'doublePoints',
						sourcePlayerId: 2,
						remainingMs: 20000,
						durationMs: 30000
					}
				],
				gameKey: 'keyboarding',
				gameOptions: { operation: 'addition' },
				matchDurationSecs: 60,
				inputMode: 'text',
				inputPlaceholder: 'Type here...'
			}
		});
		expect(msg.type).toBe('roomState');
		if (msg.type === 'roomState') {
			expect(msg.room.players).toHaveLength(1);
			expect(msg.room.activePowerups).toHaveLength(1);
			expect(msg.room.activePowerups[0].kind).toBe('doublePoints');
			expect(msg.room.matchRemainingMs).toBe(45000);
			expect(msg.room.gameKey).toBe('keyboarding');
			expect(msg.room.matchDurationSecs).toBe(60);
			expect(msg.room.inputMode).toBe('text');
			expect(msg.room.inputPlaceholder).toBe('Type here...');
		}
	});

	it('accepts the exact promptState shape', () => {
		const msg = decode({
			type: 'promptState',
			roomCode: 'ABCD',
			playerId: 1,
			roundId: 2,
			prompt: 'hello'
		});
		expect(msg.type).toBe('promptState');
	});

	it('accepts the exact raceProgress shape', () => {
		const msg = decode({
			type: 'raceProgress',
			roomCode: 'ABCD',
			playerId: 1,
			text: 'hel'
		});
		expect(msg.type).toBe('raceProgress');
	});

	it('accepts the exact roundResult shape', () => {
		const msg = decode({
			type: 'roundResult',
			roomCode: 'ABCD',
			roundId: 2,
			winnerPlayerId: 1,
			growthAwarded: 4.0
		});
		expect(msg.type).toBe('roundResult');
	});

	it('accepts the exact wrongAnswer shape', () => {
		const msg = decode({
			type: 'wrongAnswer',
			roomCode: 'ABCD',
			playerId: 1,
			shrinkApplied: 2.0
		});
		expect(msg.type).toBe('wrongAnswer');
		if (msg.type === 'wrongAnswer') {
			expect(msg.shrinkApplied).toBe(2.0);
		}
	});

	it('accepts error with code', () => {
		const msg = decode({
			type: 'error',
			message: 'No room found',
			code: 'roomNotFound'
		});
		expect(msg.type).toBe('error');
		if (msg.type === 'error') {
			expect(msg.code).toBe('roomNotFound');
		}
	});

	it('accepts error without code (field omitted)', () => {
		const msg = decode({
			type: 'error',
			message: 'Boom'
		});
		expect(msg.type).toBe('error');
		if (msg.type === 'error') {
			expect(msg.code).toBeUndefined();
		}
	});

	it('accepts every error code the server emits', () => {
		const codes = [
			'roomNotFound',
			'invalidGameMode',
			'invalidMessageFormat',
			'invalidRejoinToken',
			'roomExpired',
			'playerNotInRoom'
		];
		for (const code of codes) {
			const msg = decode({ type: 'error', message: 'x', code });
			expect(msg.type).toBe('error');
		}
	});

	it('accepts every powerUp kind the server emits', () => {
		const kinds = ['doublePoints', 'scrambleFont', 'scoreSteal', 'ongoingScoreSteal'];
		for (const kind of kinds) {
			const msg = decode({
				type: 'powerUpOffered',
				offerId: 1,
				playerId: 1,
				kind,
				expiresInMs: 30000
			});
			expect(msg.type).toBe('powerUpOffered');
		}
	});

	it('accepts the exact powerUpOffered shape', () => {
		const msg = decode({
			type: 'powerUpOffered',
			offerId: 5,
			playerId: 2,
			kind: 'scrambleFont',
			expiresInMs: 30000
		});
		expect(msg.type).toBe('powerUpOffered');
	});

	it('accepts the exact powerUpActivated shape', () => {
		const msg = decode({
			type: 'powerUpActivated',
			offerId: 3,
			playerId: 2,
			kind: 'doublePoints',
			durationMs: 30000
		});
		expect(msg.type).toBe('powerUpActivated');
	});

	it('accepts the exact powerUpOfferExpired shape', () => {
		const msg = decode({
			type: 'powerUpOfferExpired',
			offerId: 7,
			playerId: 3,
			kind: 'doublePoints'
		});
		expect(msg.type).toBe('powerUpOfferExpired');
	});

	it('accepts the exact powerUpEffectEnded shape', () => {
		const msg = decode({
			type: 'powerUpEffectEnded',
			playerId: 1,
			kind: 'scrambleFont'
		});
		expect(msg.type).toBe('powerUpEffectEnded');
	});
});

describe('server → client wire contract rejects drift', () => {
	it('rejects snake_case tags that the server never emits', () => {
		expect(
			decodeServerMessage(
				JSON.stringify({
					type: 'room_state',
					room: {
						roomCode: 'ABCD',
						players: [],
						matchWinner: null,
						matchRemainingMs: null,
						hostPlayerId: 1,
						activePowerups: [],
						gameKey: 'keyboarding',
						gameOptions: null,
						matchDurationSecs: 60,
						inputMode: 'text',
						inputPlaceholder: ''
					}
				})
			)
		).toBeNull();
	});

	it('rejects snake_case fields in a RoomSnapshot', () => {
		expect(
			decodeServerMessage(
				JSON.stringify({
					type: 'roomState',
					room: {
						room_code: 'ABCD',
						players: [],
						match_winner: null,
						match_remaining_ms: null,
						host_player_id: 1,
						active_powerups: [],
						game_key: 'keyboarding',
						game_options: null,
						match_duration_secs: 60,
						input_mode: 'text',
						input_placeholder: ''
					}
				})
			)
		).toBeNull();
	});

	it('rejects snake_case powerup kind', () => {
		expect(
			decodeServerMessage(
				JSON.stringify({
					type: 'powerUpOffered',
					offerId: 1,
					playerId: 1,
					kind: 'double_points',
					expiresInMs: 30000
				})
			)
		).toBeNull();
	});
});
