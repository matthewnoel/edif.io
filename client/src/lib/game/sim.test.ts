import { describe, expect, it } from 'vitest';
import { blobRadius, nextBlobLayout } from './sim';

describe('nextBlobLayout', () => {
	it('returns empty layout for empty players', () => {
		expect(nextBlobLayout([], {}, 0, 800, 600)).toEqual({});
	});

	it('provides coordinates for each player', () => {
		const players = [
			{ id: 1, name: 'A', size: 20, color: '#fff', connected: true, progress: '' },
			{ id: 2, name: 'B', size: 10, color: '#000', connected: true, progress: '' }
		];
		const next = nextBlobLayout(players, {}, 16, 800, 600);
		expect(Object.keys(next)).toHaveLength(2);
		expect(next[1].x).toBeTypeOf('number');
		expect(next[2].y).toBeTypeOf('number');
	});

	it('keeps all blobs within arena bounds even in a cramped arena', () => {
		const width = 320;
		const height = 320;
		const players = Array.from({ length: 8 }, (_, i) => ({
			id: i + 1,
			name: `P${i + 1}`,
			size: 20 + i * 5,
			color: '#fff',
			connected: true,
			progress: ''
		}));
		const seeded: Record<number, { x: number; y: number }> = {};
		for (const p of players) {
			seeded[p.id] = { x: width / 2, y: height / 2 };
		}
		let layout = seeded;
		for (let step = 0; step < 400; step += 1) {
			layout = nextBlobLayout(players, layout, step * 16, width, height);
		}
		for (const p of players) {
			const r = blobRadius(p, width, height);
			const pos = layout[p.id];
			expect(pos.x).toBeGreaterThanOrEqual(r - 0.5);
			expect(pos.x).toBeLessThanOrEqual(width - r + 0.5);
			expect(pos.y).toBeGreaterThanOrEqual(r - 0.5);
			expect(pos.y).toBeLessThanOrEqual(height - r + 0.5);
		}
	});
});
