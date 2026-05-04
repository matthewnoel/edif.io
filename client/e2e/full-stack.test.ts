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
	await page.goto('/');

	// `/api/game-modes` proxies through to Rust; once the dropdown label
	// renders, the modes list is loaded and "keyboarding" is selected.
	await expect(page.getByText('Game Mode:')).toBeVisible();

	await page.getByRole('button', { name: 'Create Room' }).click();

	// `setOnWelcome` navigates into /room/CODE; the host sees Start Match.
	await page.waitForURL(/\/room\/[A-Z]{4}$/);
	const startMatch = page.getByRole('button', { name: 'Start Match' });
	await expect(startMatch).toBeVisible();
	await startMatch.click();

	// `promptState` arrives over the WebSocket; the keyboarding adapter
	// emits a single word.
	const promptEl = page.locator('.prompt strong');
	await expect(promptEl).toBeVisible();
	const prompt = (await promptEl.textContent())?.trim() ?? '';
	expect(prompt.length).toBeGreaterThan(0);

	const input = page.getByPlaceholder('Type the word; press return.');
	await input.fill(prompt);
	await input.press('Enter');

	// Server replies with `roundResult`; `latestRoundSummary` surfaces
	// "<name> won +X.X size" in the result strip.
	await expect(page.locator('.result')).toContainText(/won \+/i);
});
