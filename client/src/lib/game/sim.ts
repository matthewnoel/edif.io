import type { PlayerSnapshot } from './protocol';

export type BlobLayout = Record<number, { x: number; y: number }>;

function lerp(from: number, to: number, factor: number): number {
	return from + (to - from) * factor;
}

export function blobRadius(player: PlayerSnapshot, width: number, height: number): number {
	const arenaCap = Math.max(42, Math.min(width, height) * 0.4);
	const diameter = Math.max(42, Math.min(220, arenaCap, player.size * 4));
	return diameter / 2;
}

export function nextBlobLayout(
	players: PlayerSnapshot[],
	current: BlobLayout,
	elapsedMs: number,
	width: number,
	height: number
): BlobLayout {
	if (players.length === 0) {
		return {};
	}

	const centerX = width / 2;
	const centerY = height / 2;

	const ranked = [...players].sort((a, b) => b.size - a.size);
	const largest = ranked[0];
	const rotation = elapsedMs * 0.00035;
	const next: BlobLayout = {};

	const largestRadius = blobRadius(largest, width, height);
	const maxOrbitSpanX = Math.max(0, width / 2 - largestRadius);
	const maxOrbitSpanY = Math.max(0, height / 2 - largestRadius);

	for (let index = 0; index < ranked.length; index += 1) {
		const player = ranked[index];
		const previous = current[player.id] ?? { x: centerX, y: centerY };
		const radius = blobRadius(player, width, height);

		let targetX: number;
		let targetY: number;

		if (player.id !== largest.id) {
			const orbitIndex = index - 1;
			const orbitCount = Math.max(1, ranked.length - 1);
			const angle = rotation + (orbitIndex * Math.PI * 2) / orbitCount;
			const desiredRadius = 90 + orbitIndex * 26;
			const maxAllowedX = Math.max(0, maxOrbitSpanX - radius);
			const maxAllowedY = Math.max(0, maxOrbitSpanY - radius);
			const maxAllowed = Math.min(maxAllowedX, maxAllowedY);
			const orbitRadius = Math.max(0, Math.min(desiredRadius, maxAllowed));

			targetX = centerX + Math.cos(angle) * orbitRadius;
			targetY = centerY + Math.sin(angle) * orbitRadius;
		} else {
			targetX = centerX + Math.cos(rotation * 1.3) * 8;
			targetY = centerY + Math.sin(rotation * 1.1) * 8;
		}

		const minX = radius;
		const maxX = Math.max(minX, width - radius);
		const minY = radius;
		const maxY = Math.max(minY, height - radius);
		targetX = Math.min(maxX, Math.max(minX, targetX));
		targetY = Math.min(maxY, Math.max(minY, targetY));

		next[player.id] = {
			x: lerp(previous.x, targetX, 0.08),
			y: lerp(previous.y, targetY, 0.08)
		};
	}

	return next;
}
