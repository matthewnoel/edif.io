import { expect, test } from '@playwright/test';

/**
 * Full-stack end-to-end test.
 *
 * Boots the Rust server and Vite dev server (which proxies /ws and /api to
 * the Rust server) and walks a real browser through:
 *   - Create Room as host
 *   - Start Match
 *   - Submit one correct answer to the keyboarding prompt
 *   - Observe the server-broadcast round result render in the UI
 *
 * Keyboarding is the first registered adapter and is therefore the default
 * game mode the home page picks. Its `is_correct` rule is a verbatim string
 * match, so the prompt itself is the correct answer.
 */

test.describe('full-stack: host plays one round', () => {
	test('creates a room, starts the match, and submits one correct answer', async ({ page }) => {
		// Skip the rules dialog so it doesn't intercept the lobby's Start Match click.
		await page.addInitScript(() => {
			window.localStorage.setItem('rulesDialogDismissed', '1');
		});

		// Wait for the game-modes API so the home page populates a default
		// game mode before the user clicks Create Room.
		const gameModesPromise = page.waitForResponse(
			(r) => r.url().includes('/api/game-modes') && r.ok()
		);
		await page.goto('/');
		await gameModesPromise;
		await expect(page.getByText('Game Mode:')).toBeVisible();

		await page.getByRole('button', { name: 'Create Room' }).click();
		await page.waitForURL(/\/room\/[A-Z]{4}$/);

		const startBtn = page.getByRole('button', { name: 'Start Match' });
		await expect(startBtn).toBeVisible();
		await startBtn.click();

		const promptStrong = page.locator('.prompt strong').first();
		await expect(promptStrong).toBeVisible();
		const prompt = (await promptStrong.innerText()).trim();
		expect(prompt.length).toBeGreaterThan(0);

		const input = page.getByPlaceholder('Type the word; press return.');
		await input.fill(prompt);
		await input.press('Enter');

		await expect(page.locator('.result')).toContainText(/won \+\d+\.\d size/);
	});
});
