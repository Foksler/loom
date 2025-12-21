# Loom E2E Tests

End-to-end testing suite for loom-web using Playwright.

## Quick Start

```bash
# From workspace root
make test-e2e              # Run all tests
make test-e2e-ui           # Interactive mode
make test-e2e-debug        # Debug mode

# Or from crates/loom-web
npm run test:e2e
```

## Test Structure

```
tests/
└── e2e/
    ├── navigation.spec.ts    (11 tests) - Routing, navigation
    ├── styleguide.spec.ts    (15 tests) - Component showcase
    ├── components.spec.ts    (20 tests) - Component functionality
    ├── threads.spec.ts       (12 tests) - Thread features
    ├── streaming.spec.ts     (11 tests) - Streaming messages
    ├── visual.spec.ts        (15 tests) - Visual regression
    ├── __screenshots__/      - Visual baselines
    └── .gitignore
```

## Test Suites Overview

### Navigation (11 tests)
Tests for page routing and browser history functionality.
- Home, threads, styleguide, workspace navigation
- Back/forward buttons
- Direct URL access

### Styleguide (15 tests)
Component showcase and documentation validation.
- Primitives, chat, query, results, layout sections
- Component rendering and interactivity
- Section navigation

### Components (20 tests)
Individual component functionality testing.
- Button, TextField, Select, Modal, Tabs
- Event handling, state management
- Accessibility

### Threads (12 tests)
Thread management feature tests.
- Thread list display and loading
- Detail page navigation
- Message rendering
- Conversation display

### Streaming (11 tests)
Real-time message streaming validation.
- Prompt submission
- Streaming message display
- Stop functionality
- Message persistence

### Visual (15 tests)
Visual regression and responsive design tests.
- Page screenshots across devices
- Dark theme consistency
- Responsive layouts
- Loading/error states

## Key Features

✓ **84 total tests** across 6 suites  
✓ **Multi-browser testing** (Chromium, Firefox, WebKit)  
✓ **Visual regression** with screenshot comparison  
✓ **Responsive design** validation  
✓ **Accessibility** checks  
✓ **Error handling** and edge cases  
✓ **CI/CD ready** with GitHub Actions workflow  

## Test Requirements

- Application running at `http://localhost:3000`
- Dev server: `cargo leptos watch`
- Sample data for threads
- Styleguide routes functional

## Documentation

- **Full Guide**: [`../TESTING_GUIDE.md`](../TESTING_GUIDE.md)
- **Summary**: [`../E2E_TESTING_SUMMARY.md`](../E2E_TESTING_SUMMARY.md)
- **Quick Ref**: [`../TESTING_QUICK_REFERENCE.md`](../TESTING_QUICK_REFERENCE.md)

## Running Specific Tests

```bash
# Single test file
npm --prefix crates/loom-web run test:e2e -- navigation.spec.ts

# Matching pattern
npm --prefix crates/loom-web run test:e2e -- -g "button"

# Specific test
npm --prefix crates/loom-web run test:e2e -- -g "home page loads"

# Single browser
npm --prefix crates/loom-web run test:e2e -- --project=chromium
```

## Debugging

```bash
# Interactive UI mode
npm --prefix crates/loom-web run test:e2e:ui

# Debug mode with inspector
npm --prefix crates/loom-web run test:e2e:debug

# View HTML report
npx playwright show-report
```

## Visual Testing

```bash
# Create baseline screenshots (first run)
npm --prefix crates/loom-web run test:e2e:update-snapshots

# Compare against baselines
npm --prefix crates/loom-web run test:e2e
```

## Configuration

See [`../playwright.config.ts`](../playwright.config.ts) for:
- Browser settings
- Test timeout
- Screenshot/video capture
- Dev server configuration
- Report generation

## CI/CD

GitHub Actions workflow: [`.github/workflows/e2e-tests.yml`](../.github/workflows/e2e-tests.yml)

Runs on:
- Push to main/develop
- Pull requests to main/develop
- Multiple Node versions (18.x, 20.x)

## Best Practices

1. **Use data-testid** for reliable selectors
2. **Wait for conditions**, not fixed delays
3. **Test user workflows**, not implementation
4. **Handle optional elements** gracefully
5. **Clear test names** documenting intent
6. **Screenshot tests** for visual regression

## Maintenance

- **Weekly**: Monitor for flaky tests
- **Monthly**: Update Playwright version
- **Per feature**: Add corresponding tests
- **On UI changes**: Update visual snapshots

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Port in use | `lsof -ti:3000 \| xargs kill -9` |
| Tests timeout | Increase timeout or check server |
| Element not found | Use UI mode to inspect selectors |
| Flaky tests | Add proper waits, not delays |
| Visual diffs | Update snapshots if intentional |

See [`../TESTING_GUIDE.md`](../TESTING_GUIDE.md#troubleshooting) for more help.

## Statistics

- **Total Tests**: 84
- **Test Files**: 6
- **Browsers**: 3 (Chromium, Firefox, WebKit)
- **Typical Runtime**: 2-5 min local, 5-10 min CI
- **Coverage**: Navigation, Components, Pages, Streaming, Visual

## Related Files

```
playwright.config.ts             - Main configuration
crates/loom-web/package.json     - Test scripts & dependencies
.github/workflows/e2e-tests.yml  - CI/CD configuration
Makefile                         - Make targets
```

## Contributing

When adding features:

1. ✅ Write tests first (TDD)
2. ✅ Add `data-testid` attributes
3. ✅ Test user workflows
4. ✅ Run: `make test-e2e`
5. ✅ Update visual snapshots if UI changes
6. ✅ Verify CI passes

## Resources

- [Playwright Documentation](https://playwright.dev)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Test Debugging Guide](https://playwright.dev/docs/debug)
- [Selector Guide](https://playwright.dev/docs/locators)

---

For questions or issues, see the full documentation in `TESTING_GUIDE.md`.
