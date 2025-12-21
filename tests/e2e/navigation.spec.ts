import { test, expect } from '@playwright/test';

/**
 * Navigation Test Suite
 *
 * This test suite verifies core navigation functionality throughout the loom-web
 * application. It tests:
 * - Home page loads correctly
 * - Navigation between major routes (threads, styleguide, workspace)
 * - Back/forward button functionality
 * - Navigation menu rendering
 *
 * Purpose: Ensure navigation structure works correctly and all major routes are accessible.
 */

test.describe('Navigation', () => {
  test('home page loads', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/loom/i);
    await expect(page.locator('main')).toBeVisible();
  });

  test('home page displays welcome content', async ({ page }) => {
    await page.goto('/');
    // Check for main header or welcome message
    const heading = page.locator('h1, h2').first();
    await expect(heading).toBeVisible();
  });

  test('navigation to threads page', async ({ page }) => {
    await page.goto('/');
    // Navigate to threads
    await page.click('a[href="/threads"], button:has-text("Threads")');
    await page.waitForURL('/threads');
    await expect(page).toHaveURL(/\/threads/);
  });

  test('navigation to styleguide', async ({ page }) => {
    await page.goto('/');
    // Navigate to styleguide
    await page.click('a[href="/styleguide"], button:has-text("Styleguide")');
    await page.waitForURL('/styleguide');
    await expect(page).toHaveURL(/\/styleguide/);
  });

  test('navigation to workspace', async ({ page }) => {
    await page.goto('/');
    // Navigate to workspace
    await page.click('a[href="/workspace"], button:has-text("Workspace")');
    await page.waitForURL('/workspace');
    await expect(page).toHaveURL(/\/workspace/);
  });

  test('back button returns to previous page', async ({ page }) => {
    await page.goto('/');
    // Navigate to threads
    await page.click('a[href="/threads"], button:has-text("Threads")');
    await page.waitForURL('/threads');

    // Go back
    await page.goBack();
    await expect(page).toHaveURL('/');
  });

  test('forward button goes to next page', async ({ page }) => {
    await page.goto('/');
    // Navigate to threads
    await page.click('a[href="/threads"], button:has-text("Threads")');
    await page.waitForURL('/threads');

    // Go back, then forward
    await page.goBack();
    await page.goForward();
    await expect(page).toHaveURL(/\/threads/);
  });

  test('multiple navigation steps', async ({ page }) => {
    await page.goto('/');

    // Navigate: home -> threads -> styleguide
    await page.click('a[href="/threads"], button:has-text("Threads")');
    await page.waitForURL('/threads');

    await page.click('a[href="/styleguide"], button:has-text("Styleguide")');
    await page.waitForURL('/styleguide');

    await expect(page).toHaveURL(/\/styleguide/);

    // Go back twice
    await page.goBack();
    await expect(page).toHaveURL(/\/threads/);

    await page.goBack();
    await expect(page).toHaveURL('/');
  });

  test('direct URL navigation to threads', async ({ page }) => {
    await page.goto('/threads');
    await expect(page).toHaveURL('/threads');
    await expect(page.locator('main')).toBeVisible();
  });

  test('direct URL navigation to styleguide', async ({ page }) => {
    await page.goto('/styleguide');
    await expect(page).toHaveURL('/styleguide');
    await expect(page.locator('main')).toBeVisible();
  });

  test('direct URL navigation to workspace', async ({ page }) => {
    await page.goto('/workspace');
    await expect(page).toHaveURL('/workspace');
    await expect(page.locator('main')).toBeVisible();
  });

  test('invalid route redirects or shows error', async ({ page }) => {
    const response = await page.goto('/invalid-route-that-does-not-exist');
    // Should either redirect or show 404 - check page is still functional
    await expect(page.locator('main')).toBeVisible();
  });
});
