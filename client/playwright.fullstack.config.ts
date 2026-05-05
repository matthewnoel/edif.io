import { defineConfig } from '@playwright/test';

/**
 * Full-stack Playwright config.
 *
 * Boots the Rust server (port 4000) and the Vite dev server (port 5173).
 * Vite's dev-mode proxy forwards /ws and /api/* to the Rust server, so the
 * client speaks to a real backend without any extra reverse proxy.
 *
 * Smoke tests in `e2e/` continue to run via the default `playwright.config.ts`
 * against `npm run preview` so they keep catching production-bundle regressions.
 */
export default defineConfig({
	testDir: 'e2e-fullstack',
	timeout: 60_000,
	expect: { timeout: 10_000 },
	use: {
		baseURL: 'http://127.0.0.1:5173',
		trace: 'retain-on-failure'
	},
	webServer: [
		{
			command: 'cargo run -p server',
			cwd: '..',
			url: 'http://127.0.0.1:4000/healthz',
			timeout: 600_000,
			reuseExistingServer: !process.env.CI,
			stdout: 'pipe',
			stderr: 'pipe',
			env: {
				BIND_ADDR: '127.0.0.1:4000'
			}
		},
		{
			command: 'npm run dev -- --port 5173',
			url: 'http://127.0.0.1:5173',
			timeout: 120_000,
			reuseExistingServer: !process.env.CI,
			stdout: 'pipe',
			stderr: 'pipe'
		}
	]
});
