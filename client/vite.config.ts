import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';

export default defineConfig({
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			// Viewer language is an explicit, client-persisted choice. Dropping
			// `preferredLanguage` keeps SSR/first paint deterministically English.
			strategy: ['localStorage', 'baseLocale']
		})
	],
	server: {
		proxy: {
			'/ws': {
				target: 'ws://127.0.0.1:4000',
				ws: true
			},
			'/api': {
				target: 'http://127.0.0.1:4000'
			}
		}
	},
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
