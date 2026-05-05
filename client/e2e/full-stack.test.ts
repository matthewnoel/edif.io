import { expect, test } from '@playwright/test';

/**
 * Full-stack smoke test.
 *
 * Drives a real browser against the real Rust server through a real
 * WebSocket upgrade (browser → Vite dev `/ws` proxy → axum). This is the
 * coverage that catches regressions in axum / tokio / tokio-tungstenite /
 * browser-side WS framing that the in-process Rust integration suite
 * (which uses `tokio-tungstenite` on both ends) and the client
 * unit/contract tests can't see.
 *
 * `playwright.config.ts` boots both the Rust server and `vite dev` for
 * us; the proxy from `/ws` to `127.0.0.1:4000` lives in `vite.config.ts`.
 */

test.use({ baseURL: 'http://localhost:5173' });

test('host can create a room and win one round of keyboarding', async ({ page }) => {
	// Surface browser console + uncaught errors into the Playwright trace so a
	// CI failure is debuggable from the artifact alone (svelte-kit hydration
	// errors, WebSocket onerror messages, etc.).
	const consoleMessages: string[] = [];
	const pageErrors: string[] = [];
	page.on('console', (msg) => {
		consoleMessages.push(`[${msg.type()}] ${msg.text()}`);
	});
	page.on('pageerror', (err) => {
		pageErrors.push(err.stack ?? err.message);
	});

	// Sanity-check the proxy chain BEFORE driving the UI. If this fails the
	// trace will tell us "the proxy is broken" rather than a misleading
	// "expected element not visible" further down.
	const apiResp = await page.request.get('/api/game-modes');
	expect(apiResp.status(), 'GET /api/game-modes via vite-dev proxy').toBe(200);
	const modes = (await apiResp.json()) as { key: string }[];
	expect(modes.map((m) => m.key)).toContain('keyboarding');

	await page.goto('/');

	// `/api/game-modes` is fetched on mount; once the dropdown label renders,
	// modes are loaded and "keyboarding" is selected.
	await expect(page.getByText('Game Mode:')).toBeVisible({ timeout: 15_000 });

	await page.getByRole('button', { name: 'Create Room' }).click();

	// `setOnWelcome` navigates into /room/CODE; the host sees Start Match.
	await page.waitForURL(/\/room\/[A-Z]{4}$/, { timeout: 15_000 });
	const startMatch = page.getByRole('button', { name: 'Start Match' });
	await expect(startMatch).toBeVisible({ timeout: 15_000 });
	await startMatch.click();

	// `promptState` arrives over the WebSocket; the keyboarding adapter
	// emits a single word.
	const promptEl = page.locator('.prompt strong');
	await expect(promptEl).toBeVisible({ timeout: 15_000 });
	const prompt = (await promptEl.textContent())?.trim() ?? '';
	expect(prompt.length).toBeGreaterThan(0);

	const input = page.getByPlaceholder('Type the word; press return.');
	await input.fill(prompt);
	await input.press('Enter');

	// Server replies with `roundResult`; `latestRoundSummary` surfaces
	// "<name> won +X.X size" in the result strip.
	await expect(page.locator('.result')).toContainText(/won \+/i, { timeout: 15_000 });

	// Attach captured browser output to the test result so it shows up in the
	// HTML report even when assertions pass.
	await test
		.info()
		.attach('console.log', { body: consoleMessages.join('\n'), contentType: 'text/plain' });
	await test
		.info()
		.attach('page-errors.log', { body: pageErrors.join('\n\n'), contentType: 'text/plain' });
});
