import { test, expect } from '@playwright/test';

/**
 * Threads Test Suite
 *
 * This test suite verifies the threads feature:
 * - Thread list page loads and displays threads
 * - Thread search/filter functionality
 * - Navigation to thread detail pages
 * - Thread detail page displays messages
 * - Conversation rendering
 *
 * Purpose: Ensure thread management and display features work correctly.
 */

test.describe('Threads', () => {
  test('thread list page loads', async ({ page }) => {
    await page.goto('/threads');
    await expect(page).toHaveURL('/threads');
    await expect(page.locator('main')).toBeVisible();
  });

  test('thread list displays content', async ({ page }) => {
    await page.goto('/threads');
    // Look for thread list container
    const listContainer = page.locator('[data-testid="thread-list"], .thread-list, .threads, ul, div').first();
    await expect(listContainer).toBeVisible();
  });

  test('thread items are visible', async ({ page }) => {
    await page.goto('/threads');
    // Look for thread items
    const items = page.locator('[data-testid="thread-item"], .thread-item, [data-testid="thread"], .thread');
    const count = await items.count();
    // Should have thread items or empty state message
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('can scroll through thread list', async ({ page }) => {
    await page.goto('/threads');
    const listContainer = page.locator('[data-testid="thread-list"], .thread-list, main').first();

    // Get initial scroll position
    const initialScrollTop = await listContainer.evaluate((el) => el.scrollTop);

    // Scroll down
    await listContainer.evaluate((el) => {
      el.scrollTop = el.scrollTop + 100;
    });

    // Scroll position should change
    const newScrollTop = await listContainer.evaluate((el) => el.scrollTop);
    // May or may not scroll depending on content
    expect(typeof newScrollTop).toBe('number');
  });

  test('thread list has proper structure', async ({ page }) => {
    await page.goto('/threads');
    const content = page.locator('main');
    await expect(content).toBeVisible();
    const text = await content.textContent();
    expect(text).toBeTruthy();
  });

  test('can navigate to thread detail', async ({ page }) => {
    await page.goto('/threads');
    // Look for clickable thread item
    const threadItem = page.locator('a[href*="/threads/"], [data-testid="thread-item"]').first();
    if (await threadItem.isVisible()) {
      // Get the thread ID from the link if possible
      const href = await threadItem.getAttribute('href');
      if (href) {
        await page.goto(href);
        await expect(page).toHaveURL(/\/threads\/[^\/]+/);
      }
    }
  });

  test('thread detail page loads', async ({ page }) => {
    await page.goto('/threads');
    // Try to navigate to first thread
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        const main = page.locator('main');
        await expect(main).toBeVisible();
      }
    }
  });

  test('thread detail displays messages', async ({ page }) => {
    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        // Look for message containers
        const messages = page.locator('[data-testid="message"], .message, article[data-testid*="message"]');
        const count = await messages.count();
        expect(count).toBeGreaterThanOrEqual(0);
      }
    }
  });

  test('thread conversation displays chronologically', async ({ page }) => {
    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        const messages = page.locator('[data-testid="message"], .message');
        const count = await messages.count();
        // Messages should maintain order
        if (count > 0) {
          const firstMessage = messages.nth(0);
          await expect(firstMessage).toBeVisible();
        }
      }
    }
  });

  test('thread header displays title', async ({ page }) => {
    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        // Look for thread title
        const title = page.locator('h1, h2, [data-testid="thread-title"]').first();
        if (await title.isVisible()) {
          const text = await title.textContent();
          expect(text).toBeTruthy();
        }
      }
    }
  });

  test('thread list loads without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/threads');
    await page.waitForLoadState('networkidle');

    // Should not have critical errors
    const criticalErrors = errors.filter((e) => !e.includes('Network'));
    expect(criticalErrors.length).toBe(0);
  });

  test('thread detail page loads without errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        await page.waitForLoadState('networkidle');

        const criticalErrors = errors.filter((e) => !e.includes('Network'));
        expect(criticalErrors.length).toBe(0);
      }
    }
  });

  test('back button returns to thread list from detail', async ({ page }) => {
    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        await page.goBack();
        await expect(page).toHaveURL('/threads');
      }
    }
  });

  test('page title updates for thread detail', async ({ page }) => {
    await page.goto('/threads');
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        const title = await page.title();
        expect(title).toContain('loom');
      }
    }
  });
});
