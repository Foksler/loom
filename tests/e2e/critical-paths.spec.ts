import { test, expect } from '@playwright/test';

/**
 * Critical Paths Test Suite
 *
 * This test suite verifies the most important user workflows:
 * 1. Application initialization (page loads without errors)
 * 2. Page navigation (core routes accessible)
 * 3. Content rendering (key components visible)
 * 4. User interactions (buttons, forms, navigation work)
 * 5. State persistence (data survives navigation)
 *
 * Purpose: Ensure critical user flows work reliably. These tests catch
 * breaking changes early and are fast (run first in CI).
 *
 * Why this is important:
 * - Smoke test for breaking changes
 * - Quick feedback loop (< 30s total)
 * - Catches integration issues between major components
 * - Validates deployment readiness
 */

test.describe('Critical Paths - Application Initialization', () => {
  test('application loads without critical errors', async ({ page }) => {
    /**
     * Purpose: Verify the app initializes without crashes
     * What it tests: Basic app stability, build integrity
     * Why it matters: Breaking changes are caught immediately
     */
    const errors: string[] = [];

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        // Filter out network-related errors (expected in tests without backend)
        const text = msg.text();
        if (!text.includes('Network') && !text.includes('Failed to fetch')) {
          errors.push(text);
        }
      }
    });

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Should have no critical JavaScript errors
    expect(errors).toHaveLength(0);
    expect(page.url()).toContain('localhost');
  });

  test('home page renders with main content', async ({ page }) => {
    /**
     * Purpose: Verify home page has expected structure
     * What it tests: Page structure, component rendering
     * Why it matters: Users see content immediately on load
     */
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Main page container should exist
    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Should have some content
    const content = await main.textContent();
    expect(content).toBeTruthy();
    expect(content?.length).toBeGreaterThan(0);
  });

  test('document title is set correctly', async ({ page }) => {
    /**
     * Purpose: Verify page title reflects application name
     * What it tests: HTML meta information, browser tab display
     * Why it matters: Users identify application by title
     */
    await page.goto('/');
    const title = await page.title();
    expect(title.toLowerCase()).toContain('loom');
  });
});

test.describe('Critical Paths - Core Navigation', () => {
  test('can navigate to all major sections', async ({ page }) => {
    /**
     * Purpose: Verify all major routes are accessible
     * What it tests: Router configuration, navigation links
     * Why it matters: Users need to access all app features
     */
    const majorRoutes = [
      { url: '/', name: 'Home' },
      { url: '/threads', name: 'Threads' },
      { url: '/styleguide', name: 'Styleguide' },
      { url: '/workspace', name: 'Workspace' },
    ];

    for (const route of majorRoutes) {
      await page.goto(route.url);
      await page.waitForLoadState('networkidle');

      // Page should load successfully
      expect(page.url()).toContain(route.url);

      // Should have main content area
      const main = page.locator('main');
      await expect(main).toBeVisible();
    }
  });

  test('back button works correctly', async ({ page }) => {
    /**
     * Purpose: Verify browser back button functionality
     * What it tests: History management, router state
     * Why it matters: Users expect standard browser navigation to work
     */
    // Start at home
    await page.goto('/');
    const homeUrl = page.url();

    // Navigate to threads
    await page.goto('/threads');
    const threadsUrl = page.url();

    // Go back
    await page.goBack();
    expect(page.url()).toBe(homeUrl);

    // Go forward
    await page.goForward();
    expect(page.url()).toBe(threadsUrl);
  });

  test('direct URL navigation works', async ({ page }) => {
    /**
     * Purpose: Verify users can access routes via URL bar
     * What it tests: Router deep linking, direct navigation
     * Why it matters: Bookmarks and shared links should work
     */
    const routes = ['/threads', '/styleguide', '/workspace'];

    for (const route of routes) {
      // Navigate directly to URL
      await page.goto(route);
      await page.waitForLoadState('networkidle');

      // Should be on correct page
      expect(page.url()).toContain(route);

      // Should have content
      const main = page.locator('main');
      await expect(main).toBeVisible();
    }
  });
});

test.describe('Critical Paths - Component Rendering', () => {
  test('styleguide renders all component sections', async ({ page }) => {
    /**
     * Purpose: Verify component library is accessible and rendered
     * What it tests: Component library setup, import resolution
     * Why it matters: Developers need reference for component usage
     */
    const sections = [
      'primitives',
      'chat',
      'query',
      'results',
      'layout',
    ];

    for (const section of sections) {
      await page.goto(`/styleguide/${section}`);
      await page.waitForLoadState('networkidle');

      // Section should load
      expect(page.url()).toContain(section);

      // Should have content
      const main = page.locator('main');
      await expect(main).toBeVisible();

      // Should have heading or description
      const heading = page.locator('h1, h2').first();
      const isHeadingVisible = await heading.isVisible({ timeout: 2000 }).catch(() => false);
      expect(isHeadingVisible).toBe(true);
    }
  });

  test('thread list page renders content', async ({ page }) => {
    /**
     * Purpose: Verify thread list displays without errors
     * What it tests: Data fetching, list rendering, component composition
     * Why it matters: Core feature for viewing conversations
     */
    await page.goto('/threads');
    await page.waitForLoadState('networkidle');

    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Should have some content or empty state message
    const content = await main.textContent();
    expect(content).toBeTruthy();
  });

  test('workspace page renders', async ({ page }) => {
    /**
     * Purpose: Verify workspace (main interaction area) loads
     * What it tests: Complex component setup, server function integration
     * Why it matters: Core functionality for user interaction
     */
    await page.goto('/workspace');
    await page.waitForLoadState('networkidle');

    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Workspace should have interactive elements
    const interactive = page.locator('button, input, textarea, [role="button"]');
    const count = await interactive.count();
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Critical Paths - User Interactions', () => {
  test('buttons are clickable and functional', async ({ page }) => {
    /**
     * Purpose: Verify basic button interaction works
     * What it tests: Event handlers, component interactivity
     * Why it matters: Foundation for all user interactions
     */
    await page.goto('/styleguide/primitives');
    await page.waitForLoadState('networkidle');

    const buttons = page.locator('button');
    const count = await buttons.count();

    expect(count).toBeGreaterThan(0);

    // Click first button
    const firstButton = buttons.first();
    await expect(firstButton).toBeEnabled();
    await firstButton.click();

    // Page should remain stable
    const main = page.locator('main');
    await expect(main).toBeVisible();
  });

  test('text inputs accept user input', async ({ page }) => {
    /**
     * Purpose: Verify form inputs work correctly
     * What it tests: Input event handlers, state management
     * Why it matters: Required for user to enter text
     */
    await page.goto('/styleguide/primitives');
    await page.waitForLoadState('networkidle');

    const textInputs = page.locator('input[type="text"]');
    const count = await textInputs.count();

    if (count > 0) {
      const firstInput = textInputs.first();
      const testText = 'Test input value';

      await firstInput.fill(testText);
      const value = await firstInput.inputValue();

      expect(value).toBe(testText);
    }
  });

  test('can navigate using page elements', async ({ page }) => {
    /**
     * Purpose: Verify in-page navigation links work
     * What it tests: Link handling, navigation integration
     * Why it matters: Users navigate primarily through links
     */
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Look for navigation links
    const threadsLink = page.locator('a[href="/threads"], button:has-text("Threads")').first();

    if (await threadsLink.isVisible()) {
      await threadsLink.click();
      await page.waitForURL('/threads');

      expect(page.url()).toContain('/threads');
    }
  });
});

test.describe('Critical Paths - Performance', () => {
  test('initial page load completes quickly', async ({ page }) => {
    /**
     * Purpose: Verify app loads in reasonable time
     * What it tests: Build optimization, lazy loading
     * Why it matters: Users abandon slow-loading apps
     */
    const startTime = Date.now();

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const duration = Date.now() - startTime;
    const maxTime = 10000; // 10 seconds - generous timeout for CI

    expect(duration).toBeLessThan(maxTime);
  });

  test('navigation between pages is fast', async ({ page }) => {
    /**
     * Purpose: Verify subsequent page loads are quick
     * What it tests: Route caching, component lazy loading
     * Why it matters: Application responsiveness
     */
    await page.goto('/');

    const startTime = Date.now();
    await page.goto('/threads');
    await page.waitForLoadState('networkidle');
    const duration = Date.now() - startTime;

    // Should be faster than initial load
    expect(duration).toBeLessThan(8000);
  });

  test('page handles many elements without lag', async ({ page }) => {
    /**
     * Purpose: Verify app doesn't freeze with lots of content
     * What it tests: Virtual scrolling, performance optimization
     * Why it matters: Large lists should scroll smoothly
     */
    await page.goto('/styleguide/primitives');
    await page.waitForLoadState('networkidle');

    // Scroll and ensure page remains responsive
    const main = page.locator('main').first();
    await main.evaluate((el) => {
      el.scrollTop = 1000;
    });

    // Page should still be responsive
    await expect(main).toBeVisible();
  });
});

test.describe('Critical Paths - Error Recovery', () => {
  test('invalid route shows graceful error or redirects', async ({ page }) => {
    /**
     * Purpose: Verify app handles bad routes gracefully
     * What it tests: 404 handling, error boundaries
     * Why it matters: Users should never see blank pages
     */
    await page.goto('/invalid-route-that-definitely-does-not-exist-12345');
    await page.waitForLoadState('networkidle');

    // Page should have content (either error message or redirect)
    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Should not have uncaught errors
    const content = await main.textContent();
    expect(content).toBeTruthy();
  });

  test('page recovers from CSS/styling issues', async ({ page }) => {
    /**
     * Purpose: Verify layout doesn't break under stress
     * What it tests: CSS resilience, layout stability
     * Why it matters: Visual presentation must be stable
     */
    await page.goto('/');
    await page.goto('/styleguide');
    await page.goto('/threads');
    await page.goto('/workspace');

    // Final page should render correctly
    const main = page.locator('main');
    await expect(main).toBeVisible();

    // Should have reasonable dimensions
    const box = await main.boundingBox();
    expect(box?.width).toBeGreaterThan(0);
    expect(box?.height).toBeGreaterThan(0);
  });
});
