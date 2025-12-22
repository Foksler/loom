# E2E Test Setup Guide

This guide covers setting up and configuring the Playwright E2E test suite for loom-web.

## Prerequisites

- Node.js 16+ (via devenv/nix or system installation)
- npm or pnpm
- loom-web built and ready
- Port 3000 available for dev server

## Installation

### 1. Install Dependencies

From the workspace root:

```bash
cd crates/loom-web
npm install
# or
pnpm install
```

This installs:
- `@playwright/test@1.40.0` - Testing framework
- Build tools (tailwindcss, postcss, autoprefixer)

### 2. Install Playwright Browsers

The first test run will download browsers, or install explicitly:

```bash
npx playwright install
```

This downloads:
- Chromium (~150 MB)
- Firefox (~60 MB)
- WebKit (~80 MB)

**Note**: Use `npx playwright install chromium` to install only Chromium (faster).

### 3. Verify Installation

```bash
npx playwright --version
# Should output: Version X.X.X
```

## Configuration

### Playwright Configuration

The `playwright.config.ts` file in the workspace root is already configured:

```typescript
testDir: './tests/e2e',              // Test location
baseURL: 'http://localhost:3000',    // App URL
webServer: {
  command: 'cargo leptos watch',     // Start command
  url: 'http://localhost:3000',
  reuseExistingServer: !process.env.CI,  // Reuse in dev
}
```

**Customize if needed**:
- Change `baseURL` if app runs on different port
- Change `webServer.command` if using different build tool
- Adjust timeouts in `use` section if needed

### Environment Variables

Optional environment variables:

```bash
export DEBUG=pw:api      # Enable Playwright API logging
export CI=true           # Run in CI mode (no browser reuse)
export HEADED=1          # Run tests in headed mode (see browser)
```

## Running Tests

### From Workspace Root

```bash
# All tests (uses Makefile)
make test-e2e

# Interactive UI mode
make test-e2e-ui

# Debug mode
make test-e2e-debug
```

### From loom-web Directory

```bash
cd crates/loom-web

# All tests
npm run test:e2e

# Interactive UI mode
npm run test:e2e:ui

# Debug mode
npm run test:e2e:debug

# With options
npm run test:e2e -- --grep "Critical"  # Run only Critical tests
npm run test:e2e -- tests/e2e/critical-paths.spec.ts  # Single file
npm run test:e2e -- --headed --workers=1  # Headed, single worker
```

## Build Requirements

Before running E2E tests, the app must be built and running:

### Option 1: Auto-Start (Recommended)

Playwright starts the dev server automatically:

```bash
npm run test:e2e
```

The `webServer` configuration in `playwright.config.ts` handles this.

### Option 2: Manual Start

Start the dev server in one terminal:

```bash
cargo leptos watch
# Should print: "Web server running on http://localhost:3000"
```

Then run tests in another terminal:

```bash
npm run test:e2e
```

With `reuseExistingServer: true`, tests reuse the running server instead of starting a new one.

### Option 3: Production Build

For more realistic testing:

```bash
# Build for production
cargo leptos build --release

# Run production server (adjust command for your setup)
cargo run --release

# Run tests
npm run test:e2e
```

Then update `webServer.command` in `playwright.config.ts` or set `webServer: undefined` to skip server startup.

## Troubleshooting

### Tests won't start - "Failed to launch browser"

**Cause**: Browsers not installed

**Fix**:
```bash
npx playwright install
```

### Tests hang or timeout

**Cause**: Dev server not starting properly

**Fixes**:
1. Check `webServer.command` in `playwright.config.ts`
2. Increase timeout: `timeout: 60 * 1000` (seconds * 1000)
3. Manual start: Run dev server separately, set `webServer: undefined`

### Port 3000 already in use

**Cause**: Dev server already running or other service on port

**Fixes**:
1. Kill existing process: `lsof -i :3000` then `kill <PID>`
2. Use different port: Change `baseURL` to `http://localhost:3001`
3. Set environment: `PORT=3001 npm run test:e2e`

### Browser downloads very slow

**Cause**: Network bandwidth limited, downloading all 3 browsers

**Fixes**:
1. Download single browser: `npx playwright install chromium`
2. Update `playwright.config.ts` projects to single browser:
   ```typescript
   projects: [
     { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
     // comment out firefox and webkit
   ]
   ```

### Tests fail with "element not found"

**Cause**: Selectors don't match actual DOM

**Fixes**:
1. Verify app is actually running: `curl http://localhost:3000`
2. Check selector in browser DevTools
3. Inspect actual DOM: `npx playwright codegen http://localhost:3000`
4. Look for test-specific selectors in page source

### Tests pass locally but fail in CI

**Causes**: Timing, environment differences

**Fixes**:
1. Increase timeouts:
   ```typescript
   use: {
     navigationTimeout: 30000,
     actionTimeout: 10000,
   }
   ```
2. Add explicit waits:
   ```typescript
   await page.waitForLoadState('networkidle');
   ```
3. Run with single worker: `workers: 1` in CI

## Development Workflow

### Writing Tests

1. **Start with fixtures**:
   ```typescript
   import { test, expect } from '@playwright/test';
   
   test('my new test', async ({ page }) => {
     await page.goto('/threads');
     await expect(page).toHaveURL('/threads');
   });
   ```

2. **Use Playwright inspector**:
   ```bash
   npx playwright codegen http://localhost:3000
   ```
   This opens browser where you can interact, and Playwright records code.

3. **Debug in test**:
   ```bash
   npm run test:e2e -- --debug
   ```

### Updating Tests

When UI changes:

1. Update selectors in `tests/e2e/fixtures.ts`
2. Changes apply to all tests automatically
3. Or update individual test selector

When routes change:

1. Update URL in test
2. Update `selectors.navigation` in fixtures if needed

### Adding New Tests

1. Create `.spec.ts` file in `tests/e2e/`
2. Import fixtures:
   ```typescript
   import { test, expect } from '@playwright/test';
   ```
3. Write test cases
4. Run: `npm run test:e2e -- tests/e2e/your-test.spec.ts`

## CI/CD Integration

### GitHub Actions

Add to `.github/workflows/e2e.yml`:

```yaml
name: E2E Tests
on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: ./.github/actions/setup-workspace  # Or your setup action
      
      - name: Install dependencies
        run: cd crates/loom-web && npm ci
      
      - name: Install Playwright
        run: cd crates/loom-web && npx playwright install --with-deps
      
      - name: Run E2E tests
        run: make test-e2e
        env:
          CI: true
      
      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

### GitLab CI

Add to `.gitlab-ci.yml`:

```yaml
e2e:test:
  image: mcr.microsoft.com/playwright:v1.40.0-focal
  script:
    - cd crates/loom-web
    - npm ci
    - npm run test:e2e
  artifacts:
    when: always
    paths:
      - playwright-report/
    reports:
      junit: playwright-report/junit.xml
  allow_failure: true
```

## Performance Tips

### Speed Up Tests

1. **Run only critical paths first**:
   ```bash
   npm run test:e2e -- --grep "Critical"
   ```

2. **Use single browser**:
   Update `playwright.config.ts` to test only Chromium

3. **Parallel workers**:
   ```bash
   npm run test:e2e -- --workers=4  # Default is workers per CPU
   ```

4. **Skip video/trace collection**:
   ```typescript
   use: {
     video: 'off',
     trace: 'off',
   }
   ```

5. **Headed mode can be slower**:
   ```bash
   npm run test:e2e -- --headed  # Not recommended for CI
   ```

### Reduce CI Time

```typescript
// playwright.config.ts
export default defineConfig({
  fullyParallel: true,
  workers: process.env.CI ? 4 : undefined,  // More workers in CI
  retries: process.env.CI ? 1 : 0,          // Fewer retries
  use: {
    video: 'retain-on-failure',  // Not 'retain-all'
    trace: 'on-first-retry',     // Not 'on'
  },
  projects: [
    // Test only Chromium in CI, run Firefox/WebKit nightly
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
```

## Next Steps

- [Read Test Suite Overview](./README.md)
- [View Example Tests](./critical-paths.spec.ts)
- [Learn Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Debug Failing Tests](https://playwright.dev/docs/debug)

## Support

For issues:
1. Check this guide
2. Review [Playwright docs](https://playwright.dev)
3. Check existing tests for examples
4. See main [README.md](./README.md) for test structure

---

**Last Updated**: December 2024
**Playwright Version**: 1.40.0+
**Node Version**: 16+
