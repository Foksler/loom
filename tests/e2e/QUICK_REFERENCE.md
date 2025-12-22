# E2E Tests - Quick Reference

## Run Tests

```bash
# All tests
make test-e2e              # Runs tests via Makefile
npm run test:e2e           # From crates/loom-web directory

# Interactive modes
make test-e2e-ui           # Visual test explorer
make test-e2e-debug        # Step through tests

# Specific tests
npm run test:e2e -- --grep "Critical"              # Match test name
npm run test:e2e -- tests/e2e/critical-paths.spec.ts  # Single file

# With options
npm run test:e2e -- --headed --workers=1           # Headed, 1 worker
npm run test:e2e -- --browsers=chromium             # Chrome only
npm run test:e2e -- --update-snapshots              # Update snapshots
```

## Test Files at a Glance

| File | What It Tests | Run Time |
|------|---------------|----------|
| `critical-paths.spec.ts` | **Must pass** - App loads, navigation, interactions | ~10s |
| `navigation.spec.ts` | Route navigation, browser history | ~5s |
| `components.spec.ts` | Button, input, modal, tab behavior | ~8s |
| `threads.spec.ts` | Thread list and detail pages | ~6s |
| `streaming.spec.ts` | Chat message submission and display | ~8s |
| `styleguide.spec.ts` | Component library pages | ~5s |
| `visual.spec.ts` | Visual regression checks | ~7s |

## Common Test Patterns

### Simple Navigation
```typescript
test('navigate to threads', async ({ page }) => {
  await page.goto('/threads');
  await expect(page).toHaveURL('/threads');
});
```

### Element Interaction
```typescript
test('click button', async ({ page }) => {
  await page.goto('/');
  const button = page.locator('button').first();
  await expect(button).toBeVisible();
  await expect(button).toBeEnabled();
  await button.click();
});
```

### Form Input
```typescript
test('fill form', async ({ page }) => {
  const input = page.locator('input[type="text"]');
  await input.fill('test value');
  await expect(input).toHaveValue('test value');
});
```

### Wait for Element
```typescript
test('wait for async element', async ({ page }) => {
  await page.goto('/');
  const element = page.locator('[data-testid="async"]');
  await element.waitFor({ state: 'visible', timeout: 5000 });
  await expect(element).toBeVisible();
});
```

### Check Console for Errors
```typescript
test('no console errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      // Skip network errors
      if (!msg.text().includes('Network')) {
        errors.push(msg.text());
      }
    }
  });
  
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  
  expect(errors).toHaveLength(0);
});
```

## Key Selectors (from fixtures.ts)

```typescript
// Navigation
selectors.navigation.threadsLink        // Threads link
selectors.navigation.styleguideLink     // Styleguide link
selectors.navigation.workspaceLink      // Workspace link
selectors.navigation.homeLink           // Home link

// Threads
selectors.threads.listContainer         // Thread list
selectors.threads.threadItem            // Single thread
selectors.threads.messageContainer      // Message element

// Chat
selectors.chat.composer                 // Message input
selectors.chat.sendButton               // Send button
selectors.chat.stopButton               // Stop button
selectors.chat.streamingMessage         // Streaming message

// Components
selectors.components.button             // Button element
selectors.components.input              // Text input
selectors.components.modal              // Modal dialog
selectors.components.tabs               // Tab elements
```

## Debugging

### View Test Report
```bash
npx playwright show-report
```

### Debug Specific Test
```bash
npm run test:e2e -- tests/e2e/critical-paths.spec.ts --debug
# Use Step button in inspector to step through
```

### Record Test
```bash
npx playwright codegen http://localhost:3000
# Interact with page, Playwright records code
```

### Show Test Code Generation
```bash
npx playwright test --codegen
```

### View Trace
```bash
npx playwright show-trace test-results/*/trace.zip
```

## Troubleshooting

### Tests Won't Start
```bash
# Check if browsers installed
npx playwright install

# Check if server is running
curl http://localhost:3000

# Manually start server
cargo leptos watch
```

### Test Fails with Timeout
```typescript
// Increase timeout for slow test
test.slow();  // 3x default timeout
test('slow test', async ({ page }) => { ... });

// Or specific timeout
test('slow test', async ({ page }) => {
  await page.goto('/slow-page', { waitUntil: 'networkidle', timeout: 30000 });
});
```

### Flaky Test (Sometimes Passes)
```typescript
// Use explicit waits instead of implicit
await page.waitForLoadState('networkidle');
await element.waitFor({ state: 'visible' });

// Check element state before interaction
await expect(button).toBeEnabled();
await button.click();
```

### Tests Pass Locally, Fail in CI
```bash
# Run with CI conditions
CI=true npm run test:e2e -- --workers=1

# Check for timing issues
npm run test:e2e -- --headed  # See what's happening
```

## Configuration Files

### playwright.config.ts
```typescript
// In workspace root
testDir: './tests/e2e',
baseURL: 'http://localhost:3000',
workers: process.env.CI ? 1 : undefined,
retries: process.env.CI ? 2 : 0,
use: {
  trace: 'on-first-retry',
  screenshot: 'only-on-failure',
  video: 'retain-on-failure',
},
webServer: {
  command: 'cargo leptos watch',
  url: 'http://localhost:3000',
  reuseExistingServer: !process.env.CI,
  timeout: 120 * 1000,
}
```

### package.json Scripts
```json
{
  "scripts": {
    "test:e2e": "playwright test",
    "test:e2e:ui": "playwright test --ui",
    "test:e2e:debug": "playwright test --debug",
    "test:e2e:update-snapshots": "playwright test --update-snapshots"
  }
}
```

### Makefile Commands
```makefile
test-e2e:           # Run tests
test-e2e-ui:        # Interactive UI
test-e2e-debug:     # Debug mode
```

## Best Practices

✅ **Do**:
- Write clear test names: `test('user can navigate to threads page')`
- Use fixtures: `test, expect, selectors, testHelpers`
- Wait explicitly: `await page.waitForLoadState()`
- Handle optional elements: `if (await el.isVisible({ timeout: 2000 }).catch(() => false))`
- Document complex tests with comments
- Keep tests focused on one thing
- Use data-testid for reliable selectors

❌ **Don't**:
- Use hardcoded delays: `await page.waitForTimeout(1000)`
- Chain too many operations without waits
- Test implementation details (CSS classes)
- Leave `.only()` in committed tests
- Make tests interdependent
- Ignore network errors in tests without mocking

## Performance Targets

- Single test: < 5 seconds
- All critical paths: < 30 seconds
- Full suite: < 3 minutes
- Full suite in CI: < 5 minutes

## Resources

- [Playwright Docs](https://playwright.dev)
- [API Reference](https://playwright.dev/docs/api/class-playwright)
- [Debugging Guide](https://playwright.dev/docs/debug)
- [Best Practices](https://playwright.dev/docs/best-practices)
- [Troubleshooting](https://playwright.dev/docs/troubleshooting)

## Test Structure

```
tests/
├── e2e/
│   ├── README.md                    # Full documentation
│   ├── SETUP.md                     # Installation & setup
│   ├── QUICK_REFERENCE.md           # This file
│   ├── fixtures.ts                  # Selectors, helpers
│   ├── critical-paths.spec.ts       # Smoke tests
│   ├── navigation.spec.ts           # Route navigation
│   ├── components.spec.ts           # Component behavior
│   ├── threads.spec.ts              # Thread features
│   ├── streaming.spec.ts            # Chat streaming
│   ├── styleguide.spec.ts           # Component library
│   ├── visual.spec.ts               # Visual regression
│   └── .gitignore
├── playwright.config.ts             # Playwright config (root)
└── README.md
```

## Next Steps

1. **First run**: `make test-e2e` or `npm run test:e2e`
2. **Debug failing**: `make test-e2e-debug`
3. **View report**: `npx playwright show-report`
4. **Write new tests**: Use `fixtures.ts` for consistency
5. **CI integration**: Add GitHub Actions workflow

---

**Updated**: December 2024
**Playwright**: 1.40.0+
**Node**: 16+
