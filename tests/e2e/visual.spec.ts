import { test, expect } from '@playwright/test';

/**
 * Visual Regression Test Suite
 *
 * This test suite captures screenshots of key pages for visual regression testing:
 * - Home page
 * - Thread list
 * - Thread detail
 * - Styleguide sections (primitives, chat, query, results, layout)
 *
 * Purpose: Catch unintended visual changes and regressions.
 * Note: First run with --update-snapshots to create baseline images.
 *
 * Usage:
 *   npm run test:e2e -- visual.spec.ts --update-snapshots  # Create baselines
 *   npm run test:e2e -- visual.spec.ts                      # Run visual tests
 */

test.describe('Visual Regression', () => {
  test('home page snapshot', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('home-page.png', {
      maxDiffPixels: 100,
    });
  });

  test('threads list page snapshot', async ({ page }) => {
    await page.goto('/threads');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('threads-list.png', {
      maxDiffPixels: 100,
    });
  });

  test('threads detail page snapshot', async ({ page }) => {
    await page.goto('/threads');
    await page.waitForLoadState('networkidle');

    // Try to navigate to first thread
    const threadLink = page.locator('a[href*="/threads/"]').first();
    if (await threadLink.isVisible()) {
      const href = await threadLink.getAttribute('href');
      if (href) {
        await page.goto(href);
        await page.waitForLoadState('networkidle');
        await expect(page).toHaveScreenshot('threads-detail.png', {
          maxDiffPixels: 100,
        });
      }
    }
  });

  test('styleguide index snapshot', async ({ page }) => {
    await page.goto('/styleguide');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-index.png', {
      maxDiffPixels: 100,
    });
  });

  test('styleguide primitives snapshot', async ({ page }) => {
    await page.goto('/styleguide/primitives');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-primitives.png', {
      maxDiffPixels: 150,
    });
  });

  test('styleguide chat snapshot', async ({ page }) => {
    await page.goto('/styleguide/chat');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-chat.png', {
      maxDiffPixels: 100,
    });
  });

  test('styleguide query snapshot', async ({ page }) => {
    await page.goto('/styleguide/query');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-query.png', {
      maxDiffPixels: 100,
    });
  });

  test('styleguide results snapshot', async ({ page }) => {
    await page.goto('/styleguide/results');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-results.png', {
      maxDiffPixels: 150,
    });
  });

  test('styleguide layout snapshot', async ({ page }) => {
    await page.goto('/styleguide/layout');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('styleguide-layout.png', {
      maxDiffPixels: 100,
    });
  });

  test('workspace page snapshot', async ({ page }) => {
    await page.goto('/workspace');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('workspace.png', {
      maxDiffPixels: 200,
    });
  });

  test('responsive design - mobile home page', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('home-mobile.png', {
      maxDiffPixels: 100,
    });
  });

  test('responsive design - tablet threads list', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto('/threads');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('threads-list-tablet.png', {
      maxDiffPixels: 100,
    });
  });

  test('responsive design - desktop workspace', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/workspace');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('workspace-desktop.png', {
      maxDiffPixels: 200,
    });
  });

  test('dark theme consistency', async ({ page }) => {
    // Set dark theme if application supports it
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark');
    });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('home-dark-theme.png', {
      maxDiffPixels: 100,
    });
  });

  test('components render consistently', async ({ page }) => {
    // Test button consistency across pages
    await page.goto('/styleguide/primitives');
    await page.waitForLoadState('networkidle');

    const buttons = page.locator('button').first();
    if (await buttons.isVisible()) {
      await expect(buttons).toHaveScreenshot('button-component.png', {
        maxDiffPixels: 10,
      });
    }
  });

  test('navigation bar consistency across pages', async ({ page }) => {
    // Take screenshots of nav from different pages
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const nav = page.locator('nav, header, [data-testid="navigation"]').first();
    if (await nav.isVisible()) {
      await expect(nav).toHaveScreenshot('nav-bar-home.png', {
        maxDiffPixels: 10,
      });
    }
  });

  test('loading states display correctly', async ({ page }) => {
    // Navigate and check for loading indicators
    await page.goto('/threads');
    // Immediately take screenshot before loading completes
    await page.screenshot({ path: 'tests/e2e/__screenshots__/threads-loading.png' });
    // Wait for content to load
    await page.waitForLoadState('networkidle');
  });

  test('error states display correctly', async ({ page }) => {
    // Try to navigate to non-existent route
    const response = await page.goto('/threads/non-existent-id').catch(() => null);
    await page.waitForLoadState('networkidle');

    const main = page.locator('main');
    if (await main.isVisible()) {
      // Even if error, page should render
      await expect(page).toHaveScreenshot('error-state.png', {
        maxDiffPixels: 100,
      });
    }
  });
});
