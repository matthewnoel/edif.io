import type { Story } from '$lib/dev/stories';
import WelcomeView from '$lib/components/views/WelcomeView.svelte';
import EditRoomView from '$lib/components/views/EditRoomView.svelte';
import GameRoomView from '$lib/components/views/GameRoomView.svelte';
import { gameModes, defaultOptionValues, players, staticBlobs, PALETTE } from '$lib/dev/fixtures';

const noop = () => {};

const EMOJI = {
	doublePoints: '\u{1F4AA}',
	scrambleFont: '\u{1F92A}',
	scoreSteal: '\u{1F422}',
	ongoingScoreSteal: '\u{1F355}'
};

const arithmetic = gameModes[0];
const arithmeticOptions = defaultOptionValues(arithmetic);

const lobbyPlayers = players(4);
const midPlayers = players(4).map((p, i) => ({ ...p, progress: `${4 + i}/20` }));
const overPlayers = players(4).map((p, i) => ({ ...p, progress: `${17 + i}/20` }));

// Full prop set for GameRoomView; individual stories override the relevant slice.
function gameRoom(over: Record<string, unknown> = {}): Record<string, unknown> {
	return {
		inLobby: false,
		isHost: true,
		gameOver: false,
		timerLabel: '0:42',
		myColor: PALETTE[0],
		myPrompt: '12 + 7',
		promptScrambled: false,
		showWaitingForPrompt: false,
		roundSummary: '',
		roundSummaryColor: '',
		promptInput: '',
		inputPlaceholder: 'Enter the solution; press return.',
		inputMode: 'decimal',
		inputDisabled: false,
		activeEffects: [],
		myPendingPowerUps: [],
		otherPendingPowerUps: [],
		powerUpToast: null,
		blobs: staticBlobs(midPlayers, 1),
		roomCode: 'WXYZ',
		copyConfirmed: false,
		visualHeight: 0,
		debug: false,
		debugOpen: false,
		debugInfo: null,
		onleave: noop,
		onedit: noop,
		onstart: noop,
		onrematch: noop,
		onrefresh: noop,
		oncopy: noop,
		oninput: noop,
		onsubmit: noop,
		...over
	};
}

function welcome(over: Record<string, unknown> = {}): Record<string, unknown> {
	return {
		gameModes,
		gameMode: 'arithmetic',
		matchDuration: '60',
		gameOptions: { ...arithmeticOptions },
		roomCode: '',
		wsUrl: 'ws://localhost:4000/ws',
		showServerUrl: false,
		errorMessage: '',
		connecting: false,
		debug: false,
		socketState: 'idle',
		lastSocketDetail: '',
		oncreate: noop,
		onjoin: noop,
		onsubmit: noop,
		...over
	};
}

function editRoom(over: Record<string, unknown> = {}): Record<string, unknown> {
	return {
		gameModes,
		gameMode: 'arithmetic',
		matchDuration: '90',
		gameOptions: { ...arithmeticOptions },
		errorMessage: '',
		canSubmit: true,
		ready: true,
		onback: noop,
		onupdate: noop,
		...over
	};
}

export const pageStories: Story[] = [
	// --- Welcome ---
	{
		id: 'welcome-default',
		title: 'Welcome',
		group: 'Pages',
		component: WelcomeView,
		props: welcome()
	},
	{
		id: 'welcome-error',
		title: 'Welcome · error',
		group: 'Pages',
		component: WelcomeView,
		props: welcome({ errorMessage: 'Room codes are 4 letters' })
	},
	{
		id: 'welcome-debug',
		title: 'Welcome · debug',
		group: 'Pages',
		component: WelcomeView,
		props: welcome({ debug: true, showServerUrl: true, socketState: 'open' })
	},

	// --- Edit Room ---
	{
		id: 'edit-ready',
		title: 'Edit Room',
		group: 'Pages',
		component: EditRoomView,
		props: editRoom()
	},
	{
		id: 'edit-not-ready',
		title: 'Edit Room · loading',
		group: 'Pages',
		component: EditRoomView,
		props: editRoom({ ready: false })
	},

	// --- Game Room ---
	{
		id: 'gameroom-lobby-host',
		title: 'Game Room · lobby (host)',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			inLobby: true,
			isHost: true,
			myPrompt: '',
			timerLabel: null,
			blobs: staticBlobs(lobbyPlayers, 1)
		})
	},
	{
		id: 'gameroom-lobby-guest',
		title: 'Game Room · lobby (guest)',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			inLobby: true,
			isHost: false,
			myPrompt: '',
			timerLabel: null,
			blobs: staticBlobs(lobbyPlayers, 2)
		})
	},
	{
		id: 'gameroom-in-match',
		title: 'Game Room · in match',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom()
	},
	{
		id: 'gameroom-waiting-prompt',
		title: 'Game Room · waiting for prompt',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({ myPrompt: '', showWaitingForPrompt: true })
	},
	{
		id: 'gameroom-scrambled',
		title: 'Game Room · scrambled prompt',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			promptScrambled: true,
			otherPendingPowerUps: [],
			blobs: staticBlobs(midPlayers, 1, { 1: EMOJI.scrambleFont })
		})
	},
	{
		id: 'gameroom-active-effects',
		title: 'Game Room · active effects',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			activeEffects: [
				{
					kind: 'doublePoints',
					emoji: EMOJI.doublePoints,
					label: '2x Points',
					fraction: 0.7,
					disablesInput: false
				},
				{
					kind: 'scoreSteal',
					emoji: EMOJI.scoreSteal,
					label: 'Blue Shell!',
					fraction: 0.4,
					disablesInput: true
				}
			]
		})
	},
	{
		id: 'gameroom-my-offers',
		title: 'Game Room · my power-up offers',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			myPendingPowerUps: [
				{ offerId: 1, emoji: EMOJI.doublePoints, ringOffset: 30 },
				{ offerId: 2, emoji: EMOJI.scoreSteal, ringOffset: 70 }
			]
		})
	},
	{
		id: 'gameroom-other-offers',
		title: 'Game Room · others vying',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			otherPendingPowerUps: [
				{
					offerId: 3,
					emoji: EMOJI.ongoingScoreSteal,
					label: 'Babbage vying for Point Eater!',
					fraction: 0.55,
					color: PALETTE[1]
				}
			]
		})
	},
	{
		id: 'gameroom-toast',
		title: 'Game Room · power-up toast',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({ powerUpToast: { emoji: EMOJI.doublePoints, label: '2x Points' } })
	},
	{
		id: 'gameroom-round-summary',
		title: 'Game Room · round summary',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({ roundSummary: 'Curie won +5.0 size', roundSummaryColor: PALETTE[2] })
	},
	{
		id: 'gameroom-game-over',
		title: 'Game Room · game over',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			gameOver: true,
			myPrompt: '',
			timerLabel: null,
			roundSummary: 'Ada wins the match',
			roundSummaryColor: PALETTE[0],
			blobs: staticBlobs(overPlayers, 1)
		})
	},
	{
		id: 'gameroom-copy-confirmed',
		title: 'Game Room · link copied',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({ copyConfirmed: true })
	},
	{
		id: 'gameroom-debug',
		title: 'Game Room · debug panel',
		group: 'Pages',
		component: GameRoomView,
		props: gameRoom({
			debug: true,
			debugOpen: true,
			debugInfo: {
				gameKey: 'arithmetic',
				roomCode: 'WXYZ',
				socket: 'open',
				inbound: 128,
				outbound: 64,
				players: 4
			}
		})
	}
];
