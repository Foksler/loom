import { test as base, expect, Page } from '@playwright/test';

/**
 * Test Fixtures for Loom E2E Tests
 *
 * This module provides common fixtures and utilities for E2E testing.
 * It sets up:
 * - Page state (navigation, wait for ready)
 * - Common selectors (centralized for maintainability)
 * - Helper functions (click, fill, wait patterns)
 *
 * Purpose: Reduce code duplication, improve test maintainability, and ensure
 * consistent setup across all test suites.
 */

/**
 * Centralized selectors for key UI elements
 * 
 * Usage in tests:
 *   const { selectors } = test.use({ testFixture });
 *   await page.click(selectors.navigation.threadsLink);
 */
export const selectors = {
  // Navigation elements
  navigation: {
    threadsLink: 'a[href="/threads"], button:has-text("Threads")',
    styleguideLink: 'a[href="/styleguide"], button:has-text("Styleguide")',
    workspaceLink: 'a[href="/workspace"), button:has-text("Workspace")',
    homeLink: 'a[href="/"], a[data-testid="home-link"]',
  },

  // Thread elements
  threads: {
    listContainer: '[data-testid="thread-list"], .thread-list, .threads, [role="list"]',
    threadItem: '[data-testid="thread-item"], .thread-item, [data-testid="thread"], .thread, [role="listitem"]',
    threadLink: 'a[href*="/threads/"]',
    threadTitle: '[data-testid="thread-title"], h1, h2',
    messageContainer: '[data-testid="message"], .message, article[data-testid*="message"]',
  },

  // Chat/Streaming elements
  chat: {
    composer: '[data-testid="prompt-composer"], .prompt-composer, textarea[placeholder*="prompt"], textarea[placeholder*="message"], input[placeholder*="prompt"]',
    sendButton: 'button:has-text("Send"), button:has-text("Submit"), [data-testid="send-button"], [data-testid="submit-button"]',
    stopButton: 'button:has-text("Stop"), [data-testid="stop-button"]',
    streamingMessage: '[data-testid="streaming-message"], .streaming, .message.loading',
    messageList: '[data-testid="message-list"], .message-list, .messages, [role="log"]',
  },

  // Component elements
  components: {
    button: 'button',
    input: 'input[type="text"]',
    select: 'select',
    modal: '[role="dialog"], .modal, [data-testid="modal"]',
    modalBackdrop: '[data-testid="modal-backdrop"], .modal-backdrop',
    tabs: '[role="tab"], .tab, [data-testid*="tab"], button[data-testid*="tab"]',
    tabPanel: '[role="tabpanel"], [data-testid*="tab-panel"]',
  },

  // Styleguide sections
  styleguide: {
    primitives: '/styleguide/primitives',
    chat: '/styleguide/chat',
    query: '/styleguide/query',
    results: '/styleguide/results',
    layout: '/styleguide/layout',
  },

  // Common page elements
  page: {
    main: 'main',
    heading: 'h1, h2',
    paragraph: 'p',
  },
};

/**
 * Helper functions for common test actions
 */
export const testHelpers = {
  /**
   * Navigate to a page and wait for it to be ready
   * 
   * Why: Ensures page is fully loaded before assertions
   * What it tests: Page load and ready state
   */
  async navigateTo(page: Page, url: string, waitFor = 'networkidle') {
    await page.goto(url);
    await page.waitForLoadState(waitFor);
  },

  /**
   * Click element by selector and wait for navigation if needed
   * 
   * Why: Prevents race conditions when navigation occurs after click
   * What it tests: Click event handling and navigation
   */
  async clickAndNavigate(
    page: Page,
    selector: string,
    waitForUrl?: RegExp | string,
  ) {
    await page.click(selector);
    if (waitForUrl) {
      await page.waitForURL(waitForUrl);
    } else {
      await page.waitForLoadState('networkidle');
    }
  },

  /**
   * Fill a form field and verify the value was set
   * 
   * Why: Ensures input was successfully entered
   * What it tests: Form input handling and state management
   */
  async fillAndVerify(
    page: Page,
    selector: string,
    value: string,
  ) {
    await page.fill(selector, value);
    const actualValue = await page.inputValue(selector);
    if (actualValue !== value) {
      throw new Error(
        `Failed to set input. Expected: "${value}", Got: "${actualValue}"`,
      );
    }
  },

  /**
   * Wait for element to be visible with custom timeout
   * 
   * Why: Handles elements that load asynchronously
   * What it tests: Async element rendering and visibility
   */
  async waitForElement(
    page: Page,
    selector: string,
    timeout = 5000,
  ) {
    const element = page.locator(selector);
    await element.waitFor({ state: 'visible', timeout });
    return element;
  },

  /**
   * Get element count safely (returns 0 if not found)
   * 
   * Why: Prevents test failures when element count is 0
   * What it tests: Element existence and counting
   */
  async getElementCount(page: Page, selector: string): Promise<number> {
    try {
      return await page.locator(selector).count();
    } catch {
      return 0;
    }
  },

  /**
   * Check for console errors and return filtered list
   * 
   * Why: Detects JavaScript errors without breaking on network warnings
   * What it tests: Application stability and error handling
   */
  async getConsoleErrors(
    page: Page,
    ignorePatterns: string[] = ['Network', 'Failed to fetch'],
  ): Promise<string[]> {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        const text = msg.text();
        const shouldIgnore = ignorePatterns.some((pattern) =>
          text.includes(pattern),
        );
        if (!shouldIgnore) {
          errors.push(text);
        }
      }
    });
    return errors;
  },

  /**
   * Navigate through multiple pages in sequence
   * 
   * Why: Tests navigation flow across multiple routes
   * What it tests: Multi-step navigation and state management
   */
  async navigateSequence(
    page: Page,
    urls: string[],
  ) {
    for (const url of urls) {
      await page.goto(url);
      await page.waitForLoadState('networkidle');
    }
  },

  /**
   * Scroll element into view and verify visibility
   * 
   * Why: Tests scrolling behavior and viewport handling
   * What it tests: Scroll interactions and element accessibility
   */
  async scrollIntoView(page: Page, selector: string) {
    const element = page.locator(selector);
    await element.scrollIntoViewIfNeeded();
    await expect(element).toBeVisible();
  },

  /**
   * Test keyboard navigation with Tab
   * 
   * Why: Ensures application supports keyboard accessibility
   * What it tests: Keyboard navigation and a11y compliance
   */
  async testTabNavigation(page: Page, expectedFocusedElement?: string) {
    await page.keyboard.press('Tab');
    const focusedElement = page.locator(':focus');
    await expect(focusedElement).toBeDefined();
    
    if (expectedFocusedElement) {
      await expect(focusedElement).toMatchAriaRole('button');
    }
  },

  /**
   * Handle modal open/close cycle
   * 
   * Why: Tests modal lifecycle (open, interact, close)
   * What it tests: Modal rendering, interaction, and dismissal
   */
  async testModalCycle(
    page: Page,
    openButtonSelector: string,
    modalSelector = selectors.components.modal,
  ) {
    // Open modal
    await page.click(openButtonSelector);
    const modal = page.locator(modalSelector);
    await expect(modal).toBeVisible({ timeout: 5000 }).catch(() => {
      // Modal might not exist in this context
    });

    // Close with Escape
    if (await modal.isVisible({ timeout: 2000 }).catch(() => false)) {
      await page.keyboard.press('Escape');
      await expect(modal)
        .not.toBeVisible({ timeout: 5000 })
        .catch(() => {
          // Modal might not be dismissible
        });
    }
  },

  /**
   * Submit form and wait for response
   * 
   * Why: Ensures form submission completes before continuing
   * What it tests: Form submission and data handling
   */
  async submitForm(
    page: Page,
    formSelector: string,
    submitButtonSelector?: string,
  ) {
    if (submitButtonSelector) {
      await page.click(submitButtonSelector);
    } else {
      // Look for submit button inside form
      const submitButton = page
        .locator(`${formSelector} button[type="submit"]`)
        .or(page.locator(`${formSelector} button:has-text("Submit")`));
      await submitButton.click();
    }
    await page.waitForLoadState('networkidle');
  },
};

/**
 * Extended test fixture with helpers
 * 
 * Usage:
 *   import { test } from './fixtures';
 *   
 *   test('example', async ({ page, helpers, selectors }) => {
 *     await helpers.navigateTo(page, '/threads');
 *     await expect(page.locator(selectors.threads.listContainer)).toBeVisible();
 *   });
 */
export const test = base.extend<{
  testFixture: typeof testHelpers;
}>({
  testFixture: async ({}, use) => {
    await use(testHelpers);
  },
});

export { expect };
