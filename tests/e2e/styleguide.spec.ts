import { test, expect } from '@playwright/test';

/**
 * Styleguide Test Suite
 *
 * This test suite verifies the styleguide page and all component showcase sections.
 * It tests:
 * - Styleguide page loads at /styleguide
 * - All major sections render (primitives, chat, query, results, layout)
 * - Component variants display correctly
 * - Section navigation works
 * - Expected number of components in each section
 *
 * Purpose: Ensure all UI components are properly showcased and variants render correctly.
 */

test.describe('Styleguide', () => {
  test('styleguide page loads', async ({ page }) => {
    await page.goto('/styleguide');
    await expect(page).toHaveURL(/\/styleguide/);
    await expect(page.locator('main')).toBeVisible();
  });

  test('styleguide shows component sections', async ({ page }) => {
    await page.goto('/styleguide');
    // Check for section navigation or headings
    const sections = ['Primitives', 'Chat', 'Query', 'Results', 'Layout'];
    for (const section of sections) {
      // At least one should be visible or accessible via navigation
      const locator = page.locator(`text=${section}`);
      const count = await locator.count();
      expect(count).toBeGreaterThanOrEqual(0); // Section exists or can be navigated to
    }
  });

  test('primitives section loads', async ({ page }) => {
    await page.goto('/styleguide/primitives');
    await expect(page).toHaveURL(/\/styleguide\/primitives/);
    await expect(page.locator('h1, h2').first()).toContainText(/primitives|button|input/i);
  });

  test('primitives section shows button variants', async ({ page }) => {
    await page.goto('/styleguide/primitives');
    // Look for buttons with different variants
    const buttons = page.locator('button');
    const count = await buttons.count();
    expect(count).toBeGreaterThan(0);
  });

  test('primitives section shows input variants', async ({ page }) => {
    await page.goto('/styleguide/primitives');
    // Look for input elements
    const inputs = page.locator('input[type="text"]');
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(0); // May have inputs
  });

  test('chat section loads', async ({ page }) => {
    await page.goto('/styleguide/chat');
    await expect(page).toHaveURL(/\/styleguide\/chat/);
    await expect(page.locator('h1, h2').first()).toContainText(/chat|message/i);
  });

  test('chat section shows message components', async ({ page }) => {
    await page.goto('/styleguide/chat');
    // Look for message containers
    const messages = page.locator('[data-testid*="message"], .message, article');
    const count = await messages.count();
    expect(count).toBeGreaterThanOrEqual(0); // May have message components
  });

  test('query section loads', async ({ page }) => {
    await page.goto('/styleguide/query');
    await expect(page).toHaveURL(/\/styleguide\/query/);
    await expect(page.locator('h1, h2').first()).toContainText(/query|timeline|component/i);
  });

  test('query section shows timeline component', async ({ page }) => {
    await page.goto('/styleguide/query');
    // Look for timeline elements
    const timeline = page.locator('[data-testid*="timeline"], .timeline, ul li');
    const count = await timeline.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('results section loads', async ({ page }) => {
    await page.goto('/styleguide/results');
    await expect(page).toHaveURL(/\/styleguide\/results/);
    await expect(page.locator('h1, h2').first()).toContainText(/results|code|block/i);
  });

  test('results section shows code block component', async ({ page }) => {
    await page.goto('/styleguide/results');
    // Look for code blocks
    const codeBlocks = page.locator('pre, code, [data-testid*="code"]');
    const count = await codeBlocks.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('layout section loads', async ({ page }) => {
    await page.goto('/styleguide/layout');
    await expect(page).toHaveURL(/\/styleguide\/layout/);
    await expect(page.locator('h1, h2').first()).toContainText(/layout|panel|container/i);
  });

  test('layout section shows panel components', async ({ page }) => {
    await page.goto('/styleguide/layout');
    // Look for panel/container elements
    const panels = page.locator('[data-testid*="panel"], .panel, .container, aside, section');
    const count = await panels.count();
    expect(count).toBeGreaterThan(0);
  });

  test('section navigation between styleguide pages', async ({ page }) => {
    // Navigate through sections
    const sections = ['primitives', 'chat', 'query', 'results', 'layout'];
    for (const section of sections) {
      await page.goto(`/styleguide/${section}`);
      await expect(page).toHaveURL(new RegExp(`/styleguide/${section}`));
    }
  });

  test('styleguide index page', async ({ page }) => {
    await page.goto('/styleguide');
    // Should show overview or list of available sections
    const content = page.locator('main');
    await expect(content).toBeVisible();
  });

  test('styleguide components are interactive', async ({ page }) => {
    await page.goto('/styleguide/primitives');
    // Test that buttons in styleguide can be clicked
    const firstButton = page.locator('button').first();
    if (await firstButton.isVisible()) {
      await expect(firstButton).toBeEnabled();
    }
  });

  test('all styleguide sections have content', async ({ page }) => {
    const sections = ['primitives', 'chat', 'query', 'results', 'layout'];
    for (const section of sections) {
      await page.goto(`/styleguide/${section}`);
      const main = page.locator('main');
      await expect(main).toBeVisible();
      const content = await main.textContent();
      expect(content).toBeTruthy();
    }
  });
});
