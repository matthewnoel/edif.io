import { defineConfig } from '@playwright/test';

const isCi = !!process.env.CI;

/**
 * Three web servers run in parallel:
 *  - `vite preview` on 4173: serves the production build for the
 *    home-page dependency-graph smoke tests in `home.test.ts`. The
 *    build itself is *not* part of this command — see the `pretest:e2e`
 *    npm script and CI's "Build client" step. Doing the build here as
 *    `npm run build && ...` races with `vite dev`'s svelte-kit sync
 *    (both touch `.svelte-kit/`).
 *  - `cargo run -p server` on 4000: the real Rust server, providing
 *    `/api/game-modes`, `/healthz`, and the `/ws` WebSocket upgrade.
 *  - `vite dev` on 5173: proxies `/ws` and `/api` to the Rust server
 *    (see `vite.config.ts`) so a real browser can play through a full
 *    round in `full-stack.test.ts`.
 */
export default defineConfig({
	testDir: 'e2e',
	reporter: isCi ? [['html', { open: 'never' }], ['list']] : 'list',
	use: {
		baseURL: 'http://localhost:4173',
		trace: 'retain-on-failure',
		screenshot: 'only-on-failure',
		video: 'retain-on-failure'
	},
	webServer: [
		{
			command: 'npm run preview',
			port: 4173,
			reuseExistingServer: !isCi,
			timeout: 60_000
		},
		{
			command: 'cargo run -p server',
			cwd: '..',
			url: 'http://127.0.0.1:4000/healthz',
			reuseExistingServer: !isCi,
			timeout: 240_000
		},
		{
			command: 'npm run dev -- --port 5173 --strictPort',
			url: 'http://localhost:5173',
			reuseExistingServer: !isCi,
			timeout: 60_000
		}
	]
});
