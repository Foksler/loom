# E2E Testing Quick Reference

## Setup (One-time)
```bash
cd crates/loom-web
npm install @playwright/test
npx playwright install
```

## Running Tests

### From Workspace Root
```bash
make test-e2e           # Run all tests
make test-e2e-ui        # Interactive UI mode
make test-e2e-debug     # Debug mode
```

### From loom-web Directory
```bash
npm run test:e2e                          # All tests
npm run test:e2e -- navigation.spec.ts    # Specific file
npm run test:e2e -- -g "button"           # Tests matching pattern
npm run test:e2e:ui                       # Interactive mode
npm run test:e2e:debug                    # Debug mode
npm run test:e2e:update-snapshots         # Update visual baselines
```

## Test Suites

| Suite | Tests | Focus |
|-------|-------|-------|
| `navigation.spec.ts` | 11 | Routing, back/forward |
| `styleguide.spec.ts` | 15 | Component showcase |
| `components.spec.ts` | 20 | Button, input, select, modal, tabs |
| `threads.spec.ts` | 12 | Thread list & detail |
| `streaming.spec.ts` | 11 | Prompt & messages |
| `visual.spec.ts` | 15 | Screenshots, responsive, themes |
| **Total** | **84** | **Full coverage** |

## File Locations

```
tests/e2e/
├── navigation.spec.ts     ← Routing tests
├── styleguide.spec.ts     ← Component showcase
├── components.spec.ts     ← Component functionality
├── threads.spec.ts        ← Thread features
├── streaming.spec.ts      ← Streaming messages
├── visual.spec.ts         ← Visual regression
├── .gitignore
└── __screenshots__/       ← Visual baselines

playwright.config.ts       ← Main config
Makefile                   ← Make targets
TESTING_GUIDE.md          ← Full documentation
```

## Common Commands

```bash
# Run all tests
npm --prefix crates/loom-web run test:e2e

# Run with filter
npm --prefix crates/loom-web run test:e2e -- -g "home"

# Interactive mode
npm --prefix crates/loom-web run test:e2e:ui

# Debug mode
npm --prefix crates/loom-web run test:e2e:debug

# Update visual snapshots
npm --prefix crates/loom-web run test:e2e:update-snapshots

# Specific browser
npm --prefix crates/loom-web run test:e2e -- --project=chromium

# View results
npx playwright show-report
```

## Debugging Tips

1. **Interactive UI Mode**
   ```bash
   npm --prefix crates/loom-web run test:e2e:ui
   ```
   Step through tests, inspect elements, view network

2. **Debug Mode**
   ```bash
   npm --prefix crates/loom-web run test:e2e:debug
   ```
   Attach playwright inspector

3. **Single Test**
   ```bash
   npm --prefix crates/loom-web run test:e2e -- -g "button renders"
   ```

4. **View Report**
   ```bash
   npx playwright show-report
   ```

## Test Patterns

### Navigation
```typescript
await page.goto('/page');
await page.click('link');
await page.waitForURL('/new-page');
```

### Components
```typescript
const button = page.locator('button').first();
await expect(button).toBeVisible();
await button.click();
```

### Forms
```typescript
await page.fill('input', 'value');
await page.click('button:has-text("Submit")');
await expect(page.locator('[data-testid="result"]')).toBeVisible();
```

### Visuals
```typescript
await expect(page).toHaveScreenshot('name.png');
```

## Useful Selectors

```typescript
// By role
page.locator('[role="button"]')

// By test ID
page.locator('[data-testid="my-button"]')

// By text
page.locator('button:has-text("Click me")')

// By placeholder
page.locator('input[placeholder*="search"]')

// By href
page.locator('a[href="/threads"]')

// Combining
page.locator('input[type="text"]').first()
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Port 3000 in use | `lsof -ti:3000 \| xargs kill -9` |
| Tests timeout | Increase timeout in config or test |
| Element not found | Use `page.pause()` or UI mode to inspect |
| Flaky tests | Add proper waits instead of `waitForTimeout()` |
| Visual diffs | Update snapshots: `npm run test:e2e:update-snapshots` |

## Checklist for New Features

- [ ] Write E2E tests first (TDD)
- [ ] Add `data-testid` to components
- [ ] Test user workflows
- [ ] Test error cases
- [ ] Run locally: `make test-e2e`
- [ ] Update visual snapshots if UI changed
- [ ] Check CI passes
- [ ] Document in PR

## Essential Files

- **Test Config**: `playwright.config.ts`
- **NPM Scripts**: `crates/loom-web/package.json`
- **Full Guide**: `TESTING_GUIDE.md`
- **Summary**: `E2E_TESTING_SUMMARY.md`

## Resources

- [Playwright Docs](https://playwright.dev)
- [Best Practices](https://playwright.dev/docs/best-practices)
- [Selectors](https://playwright.dev/docs/locators)
- [API Reference](https://playwright.dev/docs/api/class-test)

## Performance

- **Local**: 2-5 minutes
- **CI**: 5-10 minutes (with retries)
- **Parallel**: Default enabled
- **Selective**: Run single tests for quick feedback

---

See `TESTING_GUIDE.md` for complete documentation.
