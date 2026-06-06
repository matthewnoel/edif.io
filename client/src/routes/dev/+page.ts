import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';

// Client-only: the playground renders components that use performance.now / rAF /
// localStorage / clipboard, so skip SSR. The dev guard means production builds 404.
export const prerender = false;
export const ssr = false;

export function load() {
	if (!dev) {
		error(404, 'Not found');
	}
}
