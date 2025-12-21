# E2E Testing Guide - Loom Web

Comprehensive end-to-end testing suite using Playwright for the loom-web application.

## Quick Start

### Prerequisites

- Node.js 16+ and npm/yarn installed
- `cargo leptos` available for running the dev server
- All Playwright browsers installed

### Installation

```bash
# Install dependencies (run from workspace root)
cd crates/loom-web
npm install @playwright/test
npx playwright install
cd ../..
```

### Running Tests

From the workspace root:

```bash
# Run all E2E tests
npm --prefix crates/loom-web run test:e2e

# Run tests in UI mode (interactive)
npm --prefix crates/loom-web run test:e2e:ui

# Run tests in debug mode
npm --prefix crates/loom-web run test:e2e:debug

# Run specific test file
npm --prefix crates/loom-web run test:e2e -- tests/e2e/navigation.spec.ts

# Run tests matching pattern
npm --prefix crates/loom-web run test:e2e -- -g "button"

# Update visual regression snapshots (first run)
npm --prefix crates/loom-web run test:e2e:update-snapshots

# Run tests with specific browser
npm --prefix crates/loom-web run test:e2e -- --project=chromium
npm --prefix crates/loom-web run test:e2e -- --project=firefox
npm --prefix crates/loom-web run test:e2e -- --project=webkit
```

## Test Structure

All tests are located in `tests/e2e/` directory. Test files:

### 1. `tests/e2e/navigation.spec.ts` (11 tests)

**Purpose**: Verify core navigation functionality throughout the application.

**Coverage**:
- Home page loads correctly
- Navigation to threads, styleguide, workspace pages
- Back/forward button functionality
- Direct URL navigation
- Multiple navigation steps
- Invalid route handling

**Key Tests**:
- `home page loads`
- `navigation to threads page`
- `back button returns to previous page`
- `forward button goes to next page`
- `direct URL navigation to [page]`

**Why Important**: Navigation is fundamental to a web app. These tests ensure all routes are accessible and browser history works correctly.

### 2. `tests/e2e/styleguide.spec.ts` (15 tests)

**Purpose**: Verify the styleguide component showcase renders correctly.

**Coverage**:
- Styleguide page loads at `/styleguide`
- All sections render (primitives, chat, query, results, layout)
- Component variants display
- Section navigation
- Content and accessibility

**Key Tests**:
- `styleguide page loads`
- `[section] section loads` (for each section)
- `[section] section shows [component] components`
- `section navigation between styleguide pages`
- `all styleguide sections have content`

**Why Important**: The styleguide is documentation for component developers. Ensuring all sections load and display correctly is critical for maintaining the component library.

### 3. `tests/e2e/components.spec.ts` (20 tests)

**Purpose**: Test individual component functionality in isolation and integration.

**Coverage**:
- Button: click, disabled state, variants
- TextField: input, clearing, placeholder, attributes
- Select: dropdown, options, selection
- Modal: open, close, escape key, backdrop
- Tabs: rendering, switching, panels
- Common: accessibility, keyboard navigation, console errors

**Key Tests**:
- `button renders and is clickable`
- `button disabled state`
- `text field accepts input`
- `select can be interacted with`
- `modal can be opened/closed`
- `tab switching changes content`
- `components respond to keyboard navigation`
- `components have no console errors`

**Why Important**: Components are building blocks of the UI. Testing them individually ensures robust reusable components before integration.

### 4. `tests/e2e/threads.spec.ts` (12 tests)

**Purpose**: Test thread management and display features.

**Coverage**:
- Thread list page loads and displays threads
- Thread items render
- Navigation to thread detail
- Thread detail displays messages
- Conversation chronological order
- Thread title display
- Error handling

**Key Tests**:
- `thread list page loads`
- `thread items are visible`
- `can navigate to thread detail`
- `thread detail displays messages`
- `thread conversation displays chronologically`
- `thread header displays title`
- `back button returns to thread list from detail`

**Why Important**: Threads are the core feature. These tests ensure the thread list and detail views work correctly and can display conversations.

### 5. `tests/e2e/streaming.spec.ts` (11 tests)

**Purpose**: Test real-time streaming message functionality.

**Coverage**:
- Prompt submission in chat interface
- Streaming message appears and updates
- Stop button cancels stream
- Message persistence
- Composer behavior
- Multiple messages in sequence
- Message ordering

**Key Tests**:
- `can input text in prompt composer`
- `can submit prompt`
- `streaming message appears after submission`
- `stop button appears during streaming`
- `can stop streaming message`
- `message persists after streaming completes`
- `can submit multiple messages in sequence`

**Why Important**: Streaming is a key feature for real-time AI responses. These tests ensure the prompt/response flow works smoothly.

### 6. `tests/e2e/visual.spec.ts` (15 tests)

**Purpose**: Capture screenshots for visual regression testing.

**Coverage**:
- Page screenshots (home, threads, workspace, styleguide sections)
- Responsive design (mobile, tablet, desktop)
- Theme consistency (dark mode)
- Component consistency
- Loading and error states

**Key Tests**:
- `[page] snapshot`
- `responsive design - [device] [page]`
- `dark theme consistency`
- `components render consistently`
- `navigation bar consistency across pages`
- `loading states display correctly`
- `error states display correctly`

**Why Important**: Visual regressions can happen silently. Screenshot tests catch unintended style changes across browsers and viewports.

## Configuration

### `playwright.config.ts`

Key settings:

```typescript
// Run tests in parallel
fullyParallel: true

// Retry failed tests on CI only
retries: process.env.CI ? 2 : 0

// Screenshot on failure, video on failure
screenshot: 'only-on-failure'
video: 'retain-on-failure'

// Run dev server before tests
webServer: {
  command: 'cargo leptos watch',
  url: 'http://localhost:3000',
  timeout: 120000,
}

// Test all major browsers
projects: [chromium, firefox, webkit]
```

## Test Data Requirements

Some tests assume:
- Application accessible at `http://localhost:3000`
- Threads page has sample thread data available
- Styleguide routes return content
- Workspace page with prompt composer available

For local testing, ensure the dev server runs with sample data.

## Continuous Integration

### GitHub Actions Setup

Example CI configuration:

```yaml
name: E2E Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install Leptos
        run: cargo install cargo-leptos
      
      - name: Install Playwright
        run: npm --prefix crates/loom-web install && npx playwright install
      
      - name: Run E2E Tests
        run: npm --prefix crates/loom-web run test:e2e
      
      - name: Upload Results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: crates/loom-web/playwright-report/
```

## Debugging

### View Test Report

After running tests:

```bash
npx playwright show-report
```

### Run Single Test in Debug Mode

```bash
npm --prefix crates/loom-web run test:e2e:debug -- -g "home page loads"
```

### Interactive UI Mode

```bash
npm --prefix crates/loom-web run test:e2e:ui
```

Features:
- Step through tests
- Inspect elements
- Replay recordings
- View network requests

### Common Issues

**Port 3000 already in use**:
```bash
# Kill process using port 3000
lsof -ti:3000 | xargs kill -9
# Or change port in playwright.config.ts
```

**Tests timeout**:
- Increase timeout in individual tests: `test.setTimeout(60000)`
- Increase webServer timeout in config

**Flaky tests**:
- Add explicit waits: `await page.waitForLoadState('networkidle')`
- Use data-testid attributes in app code
- Avoid hardcoded delays, use proper waits

## Best Practices

### 1. Use Data Test IDs

In application code, add `data-testid` attributes:

```html
<button data-testid="submit-button">Submit</button>
<div data-testid="thread-list">...</div>
```

Then in tests:
```typescript
await page.locator('[data-testid="submit-button"]').click();
```

### 2. Wait for Content

Don't rely on fixed delays:

```typescript
// Good
await page.locator('[data-testid="message"]').waitFor();
await page.waitForLoadState('networkidle');

// Bad
await page.waitForTimeout(1000);
```

### 3. Handle Optional Elements

Not all elements may exist:

```typescript
const element = page.locator('selector');
if (await element.isVisible({ timeout: 2000 }).catch(() => false)) {
  // Element exists and is visible
  await element.click();
}
```

### 4. Test Real User Workflows

Focus on what users actually do:

```typescript
// Good - user workflow
await page.fill('textarea', 'My message');
await page.click('[data-testid="send-button"]');
await expect(page.locator('[data-testid="message"]')).toBeVisible();

// Less useful - implementation details
await expect(page.locator('.message-state-loading')).toHaveClass('active');
```

### 5. Structure Tests Clearly

```typescript
test('user submits message and sees response', async ({ page }) => {
  // Setup
  await page.goto('/workspace');
  
  // Action
  await page.fill('[data-testid="prompt"]', 'Hello');
  await page.click('[data-testid="send"]');
  
  // Assertion
  await expect(page.locator('[data-testid="message"]')).toContainText('Hello');
});
```

## Extending Tests

### Add New Test Suite

1. Create file in `tests/e2e/yourfeature.spec.ts`
2. Follow existing test structure
3. Add property-based tests for complex logic
4. Document test purpose in JSDoc comments

### Add Component Tests

```typescript
test.describe('MyComponent', () => {
  test('renders with props', async ({ page }) => {
    await page.goto('/styleguide/components');
    // Test your component
  });
});
```

### Add Visual Tests

```typescript
test('component visual consistency', async ({ page }) => {
  await page.goto('/component');
  await expect(page.locator('.component')).toHaveScreenshot('component.png');
});

// Update snapshots when intentional design changes
// npm run test:e2e:update-snapshots
```

## Performance

### Parallel Execution

Tests run in parallel by default (faster on CI):

```bash
# Run with specific number of workers
npm --prefix crates/loom-web run test:e2e -- --workers=4
```

### Selective Testing

Run only changed features during development:

```bash
# Run only navigation tests
npm --prefix crates/loom-web run test:e2e -- navigation.spec.ts

# Run only tests matching pattern
npm --prefix crates/loom-web run test:e2e -- -g "button"

# Run single test
npm --prefix crates/loom-web run test:e2e -- -g "home page loads"
```

## Test Metrics

### Current Coverage

| Suite | Tests | Coverage |
|-------|-------|----------|
| Navigation | 11 | Core routing, back/forward |
| Styleguide | 15 | Component showcase, sections |
| Components | 20 | Button, input, select, modal, tabs |
| Threads | 12 | Thread list, detail, messages |
| Streaming | 11 | Prompt, streaming, messages |
| Visual | 15 | Responsive, themes, states |
| **Total** | **84** | **Core features** |

### Test Execution Time

Typical local run: 2-5 minutes (depending on system)
CI run with retries: 5-10 minutes

## Troubleshooting

### Tests Pass Locally But Fail on CI

- Check browser version compatibility
- Verify test data availability
- Check for timezone-dependent tests
- Look for network latency issues

### Visual Tests Fail on New Machine

First run with `--update-snapshots` to establish baseline:

```bash
npm --prefix crates/loom-web run test:e2e:update-snapshots
```

Then commit snapshot files.

### Timeout Issues

Increase timeout for slow operations:

```typescript
test('slow operation', async ({ page }) => {
  test.setTimeout(60000); // 60 seconds
  // ... test code
});
```

### Element Not Found

Use playwright inspector to find element:

```bash
npm --prefix crates/loom-web run test:e2e:debug -- -g "your test"
# In debug mode, use inspector to find element selectors
```

## Additional Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Playwright Debugging](https://playwright.dev/docs/debug)
- [Playwright Inspector](https://playwright.dev/docs/inspector)

## Maintenance

### Regular Tasks

- **Weekly**: Check for flaky tests, fix timeouts
- **Monthly**: Update Playwright version
- **Per Release**: Update visual snapshots for intentional changes
- **Quarterly**: Review and add tests for new features

### Updating Snapshots

When intentional visual changes are made:

```bash
npm --prefix crates/loom-web run test:e2e:update-snapshots
git add tests/e2e/__screenshots__/
git commit -m "Update visual snapshots for [feature]"
```

## Contributing

When adding new features:

1. Write E2E tests first (TDD)
2. Implement feature
3. Verify tests pass
4. Add data-testid attributes if needed
5. Document in test JSDoc comments
6. Update visual snapshots if UI changes

## License

Same as Loom project.
