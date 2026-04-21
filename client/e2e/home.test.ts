import { expect, test } from '@playwright/test';

/**
 * Home-page smoke tests.
 *
 * These run against the production build via `npm run preview` (see
 * playwright.config.ts). They don't assume the Rust server is running —
 * their purpose is to catch regressions in the client dependency graph
 * (SvelteKit, Svelte 5 runes, Vite, Playwright itself, Prettier/ESLint
 * rules that leak into generated output, etc.) when version bumps land.
 */

test.describe('home page', () => {
	test('renders the edif.io heading', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('h1')).toHaveText('edif.io');
	});

	test('shows match duration and room code inputs', async ({ page }) => {
		await page.goto('/');
		await expect(page.getByPlaceholder('60')).toBeVisible();
		await expect(page.getByPlaceholder('ABCD')).toBeVisible();
	});

	test('room code input filters non-letters and uppercases', async ({ page }) => {
		await page.goto('/');
		const input = page.getByPlaceholder('ABCD');
		await input.fill('');
		await input.pressSequentially('ab12cd');
		await expect(input).toHaveValue('ABCD');
	});

	test('room code input caps at 4 characters', async ({ page }) => {
		await page.goto('/');
		const input = page.getByPlaceholder('ABCD');
		await input.fill('');
		await input.pressSequentially('ABCDEFGH');
		const value = await input.inputValue();
		expect(value.length).toBeLessThanOrEqual(4);
	});

	test('Create Room is enabled when room code is empty, Join Room disabled', async ({ page }) => {
		await page.goto('/');
		const createBtn = page.getByRole('button', { name: 'Create Room' });
		const joinBtn = page.getByRole('button', { name: 'Join Room' });
		await expect(createBtn).toBeEnabled();
		await expect(joinBtn).toBeDisabled();
	});

	test('entering a room code enables Join Room and disables Create Room', async ({ page }) => {
		await page.goto('/');
		const input = page.getByPlaceholder('ABCD');
		await input.fill('');
		await input.pressSequentially('WXYZ');
		const createBtn = page.getByRole('button', { name: 'Create Room' });
		const joinBtn = page.getByRole('button', { name: 'Join Room' });
		await expect(joinBtn).toBeEnabled();
		await expect(createBtn).toBeDisabled();
	});

	test('shows an error when joining with a too-short room code', async ({ page }) => {
		await page.goto('/');
		const input = page.getByPlaceholder('ABCD');
		await input.fill('');
		await input.pressSequentially('AB');
		await page.getByRole('button', { name: 'Join Room' }).click();
		await expect(page.getByText('Room codes are 4 letters')).toBeVisible();
	});

	test('match duration accepts numeric input', async ({ page }) => {
		await page.goto('/');
		const duration = page.getByPlaceholder('60');
		await duration.fill('');
		await duration.fill('30');
		await expect(duration).toHaveValue('30');
	});
});

test.describe('error routes', () => {
	test('unknown routes render the 404 page', async ({ page }) => {
		const response = await page.goto('/this-route-does-not-exist');
		expect(response?.status()).toBe(404);
		await expect(page.getByText('Something went wrong')).toBeVisible();
		await expect(page.getByText(/couldn't find that page/i)).toBeVisible();
	});
});
