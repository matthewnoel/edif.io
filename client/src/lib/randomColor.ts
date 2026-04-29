import { PALETTE } from './customization';

export function randomColor(): string {
	const idx = Math.floor(Math.random() * PALETTE.length);
	return PALETTE[idx];
}
