# Loom E2E Test Suite

Comprehensive end-to-end testing for loom-web using Playwright.

## Overview

This E2E test suite validates critical user workflows and ensures the application works correctly across different browsers and scenarios.

### Test Files

| File | Purpose | Coverage |
|------|---------|----------|
| `critical-paths.spec.ts` | **PRIORITY** - Smoke tests for essential flows | App initialization, navigation, core components, interactions, performance |
| `navigation.spec.ts` | Route navigation and browser history | URL navigation, back/forward buttons, multi-step flows |
| `components.spec.ts` | Individual component behavior | Buttons, inputs, selects, modals, tabs, accessibility |
| `threads.spec.ts` | Thread list and detail pages | Thread display, navigation, message rendering |
| `streaming.spec.ts` | Chat/message streaming functionality | Prompt submission, streaming UI, message persistence |
| `styleguide.spec.ts` | Component library documentation | All styleguide sections, component variants |
| `visual.spec.ts` | Visual regression testing | Screenshot comparisons (if enabled) |

### Test Infrastructure

| File | Purpose |
|------|---------|
| `fixtures.ts` | Reusable test fixtures, selectors, and helper functions |
| `playwright.config.ts` | Playwright configuration (browser, servers, reporting) |

## Quick Start

### Run All Tests

```bash
make test-e2e           # Run all E2E tests (headless, all browsers)
```

### Run Specific Tests

```bash
make test-e2e           # All tests
npm run test:e2e        # All tests (from loom-web directory)
npm run test:e2e -- --grep "Navigation"  # Only Navigation tests
npm run test:e2e -- tests/e2e/critical-paths.spec.ts  # Only critical paths
```

### Interactive & Debugging

```bash
make test-e2e-ui        # UI mode - interactive test explorer
make test-e2e-debug     # Debug mode - step through tests
npm run test:e2e:ui     # Alternative UI mode
npm run test:e2e:debug  # Alternative debug mode
```

### Update Visual Snapshots

```bash
npm run test:e2e:update-snapshots
```

## Test Structure

### Critical Paths (Run These First)

Critical paths are smoke tests that validate the most important user workflows. They:
- **Run fast** (< 30 seconds total)
- **Catch breaking changes** (deployment blockers)
- **Are deterministic** (no flakiness)

Categories in `critical-paths.spec.ts`:
1. **Application Initialization** - App loads without errors
2. **Core Navigation** - All major routes accessible
3. **Component Rendering** - Key components display
4. **User Interactions** - Buttons, inputs work
5. **Performance** - Pages load quickly
6. **Error Recovery** - Graceful degradation

### Full Test Suite

Additional test files provide comprehensive coverage:
- **Navigation** - Route changes, history management
- **Components** - Individual component behavior
- **Threads** - Conversation management
- **Streaming** - Real-time message updates
- **Styleguide** - Component documentation
- **Visual** - Visual regression detection

## Writing New Tests

### Use the Test Fixtures

```typescript
import { test, expect, selectors, testHelpers } from './fixtures';

test('example test', async ({ page, testFixture }) => {
  // Navigate with built-in error handling
  await testFixture.navigateTo(page, '/threads');

  // Use centralized selectors
  const threadList = page.locator(selectors.threads.listContainer);
  await expect(threadList).toBeVisible();

  // Use helper for console error checking
  const errors = await testFixture.getConsoleErrors(page);
  expect(errors).toHaveLength(0);
});
```

### Best Practices

1. **Clear test names**: Describe what is being tested
   ```typescript
   test('user can navigate from home to threads page', async ({ page }) => {
     // GOOD - specific about user action and expected result
   });
   ```

2. **Add documentation**: Explain WHY test is important
   ```typescript
   /**
    * Purpose: Verify threads load without errors
    * What it tests: Data fetching, list rendering
    * Why it matters: Core feature must be reliable
    */
   ```

3. **Use selectors consistently**: Leverage `selectors` object
   ```typescript
   // Instead of hardcoding selectors:
   // ❌ page.click('a[href="/threads"]')
   // ✅ page.click(selectors.navigation.threadsLink)
   ```

4. **Handle optional elements**: Not all features may exist
   ```typescript
   const modal = page.locator(selectors.components.modal);
   if (await modal.isVisible({ timeout: 2000 }).catch(() => false)) {
     // Modal exists, test it
   }
   ```

5. **Filter out expected errors**: Network errors in tests are normal
   ```typescript
   const errors = await testFixture.getConsoleErrors(page, [
     'Network',
     'Failed to fetch',
     'Could not load',
   ]);
   ```

## Debugging Failing Tests

### View Test Report

```bash
npx playwright show-report
```

### Run in Debug Mode

```bash
make test-e2e-debug     # Or:
npm run test:e2e:debug
```

In debug mode:
- Tests pause before each action
- Use the inspector to step through
- View DOM at each step
- Check network requests in DevTools

### Enable More Logging

```bash
# Verbose output
npm run test:e2e -- --verbose

# Show Playwright API calls
npm run test:e2e -- --debug

# Run in headed mode (see browser)
npx playwright test --headed
```

### Check Screenshots & Videos

When tests fail, Playwright captures:
- **Screenshots**: `test-results/*/test-failed-*.png`
- **Videos**: `test-results/*/video.webm`
- **Traces**: `test-results/*/trace.zip`

View trace in Playwright Trace Viewer:
```bash
npx playwright show-trace test-results/*/trace.zip
```

## Common Issues & Solutions

### Test Fails with "Timeout waiting for element"

**Problem**: Element selector is wrong or element takes too long to appear

**Solutions**:
1. Check selector in browser DevTools
2. Increase timeout: `await page.locator(sel).waitFor({ timeout: 10000 })`
3. Wait for network: `await page.waitForLoadState('networkidle')`
4. Use helper: `await testFixture.waitForElement(page, selector)`

### Test Passes Locally but Fails in CI

**Causes**: Timing issues, environment differences

**Solutions**:
1. Add explicit waits: `await page.waitForLoadState('networkidle')`
2. Increase timeouts in CI
3. Use `test.slow()` for tests that are known slow:
   ```typescript
   test.slow();  // Gives 3x timeout
   test('slow operation', async ({ page }) => { ... });
   ```

### Flaky Tests (Sometimes Pass, Sometimes Fail)

**Problem**: Race conditions or timing-dependent behavior

**Solutions**:
1. Replace implicit waits with explicit ones
2. Use `waitFor` with proper state checking
3. Verify element state before interaction:
   ```typescript
   const button = page.locator('button');
   await button.waitFor({ state: 'visible' });
   await expect(button).toBeEnabled();
   await button.click();
   ```

### Tests Break After Code Changes

**Typical causes**: Selector changes, new routing structure

**Solutions**:
1. Update selectors in `fixtures.ts` - changes apply to all tests
2. Update route URLs in tests
3. Verify component rendering requirements (new loading states?)

## Performance Targets

Tests should be fast and reliable:

| Test Type | Target Time | Timeout |
|-----------|------------|---------|
| Single test | < 5s | 30s |
| All critical paths | < 30s | 60s |
| Full suite | < 3min | 5min |
| CI with 2 retries | < 5min | 10min |

## CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests
on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Install dependencies
        run: cd crates/loom-web && npm install
      
      - name: Build loom-web
        run: cargo leptos build --release
      
      - name: Run E2E tests
        run: make test-e2e
        env:
          CI: true
      
      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: e2e-results
          path: test-results/
```

### Makefile Integration

E2E tests are integrated into the main CI workflow:

```bash
make check              # Runs: format lint build test test-e2e
make test-e2e           # Run E2E tests
make test-e2e-ui        # Interactive UI mode
make test-e2e-debug     # Debug mode
```

## Configuration

### playwright.config.ts

Key settings:

```typescript
export default defineConfig({
  testDir: './tests/e2e',              // Test file location
  fullyParallel: true,                 // Run tests in parallel
  retries: process.env.CI ? 2 : 0,    // Retry failed tests in CI
  workers: process.env.CI ? 1 : 4,    // Parallel workers
  use: {
    baseURL: 'http://localhost:3000',  // App URL
    trace: 'on-first-retry',           // Collect trace on failure
    screenshot: 'only-on-failure',      // Screenshot on failure
    video: 'retain-on-failure',         // Record video on failure
  },
  webServer: {
    command: 'cargo leptos watch',     // Start app before tests
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
```

### Browser Coverage

Tests run against:
- **Chromium** (main browser - most tests run here)
- **Firefox** (compatibility)
- **WebKit** (Safari compatibility)

Disable browsers by commenting in `playwright.config.ts`:

```typescript
projects: [
  { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  // { name: 'firefox', use: { ...devices['Desktop Firefox'] } },  // Disabled
  // { name: 'webkit', use: { ...devices['Desktop Safari'] } },    // Disabled
],
```

## Known Limitations

### Streaming Messages

- Streaming functionality requires a real backend
- Mock tests verify UI structure but not actual streaming
- Full integration tests should use test server with mock LLM

### Authentication

- Current tests assume no authentication required
- Add authentication fixtures if auth is implemented:
  ```typescript
  test.beforeEach(async ({ page }) => {
    // Login before test
    await page.goto('/login');
    await page.fill('input[type="email"]', 'test@example.com');
    await page.fill('input[type="password"]', 'test-password');
    await page.click('button[type="submit"]');
    await page.waitForURL('/workspace');
  });
  ```

### Server Dependencies

- Tests assume `cargo leptos watch` or similar server running
- Adjust `webServer.command` in `playwright.config.ts` if needed

### Network Mocking

- To mock API responses, use Playwright's route intercepting:
  ```typescript
  await page.route('**/api/threads', route => {
    route.abort('blockedbyclient');  // Block requests
    // or
    route.fulfill({ json: { threads: [] } });  // Mock response
  });
  ```

## Maintenance

### Regular Tasks

- **Monthly**: Update Playwright (`npm update @playwright/test`)
- **Per sprint**: Review failing test patterns, fix flaky tests
- **Per feature**: Add/update tests for new features
- **Per bug**: Add regression test before fixing

### Cleaning Up Tests

Old selectors after refactoring:

```bash
# Find tests referencing old classnames
grep -r "old-class-name" tests/e2e/*.ts

# Update selectors in fixtures.ts for consistency
# Update individual tests if they have specific selectors
```

### Monitor Test Health

```bash
# See which tests are slowest
npm run test:e2e -- --reporter list

# Run only failed tests from last run
npm run test:e2e -- --last-failed
```

## Resources

- [Playwright Docs](https://playwright.dev/)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Debugging Guide](https://playwright.dev/docs/debug)
- [CI/CD Guide](https://playwright.dev/docs/ci)

## Support

For issues or questions:
1. Check this README
2. Check [Playwright docs](https://playwright.dev/)
3. Review existing tests for examples
4. Check GitHub issues
5. Open a new issue with:
   - Test name and file
   - Error message
   - Steps to reproduce
   - Environment (OS, Node version)
