# E2E Tests - Quick Start

**Run Playwright tests for loom-web in 3 minutes**

---

## ⚡ Fastest Path to Tests Running

### 1. Prerequisites (1 min)

```bash
# Check you have Node.js 16+
node --version    # Should be v16+
npm --version     # Should be v8+

# If not installed, install from https://nodejs.org/
```

### 2. Install Dependencies (1 min)

```bash
cd /home/ghuntley/loom/crates/loom-web
npm install
```

### 3. Run Tests (1 min)

```bash
# Run all tests
npm run test:e2e

# Or from workspace root:
cd /home/ghuntley/loom
make test-e2e
```

---

## 🎯 Common Commands

```bash
# Run all tests (headless)
npm run test:e2e

# Interactive UI (see tests run visually)
npm run test:e2e:ui

# Debug mode (step through)
npm run test:e2e:debug

# Run specific test file
npx playwright test critical-paths.spec.ts

# Run specific test
npx playwright test -g "application loads"

# View HTML report
npx playwright show-report

# Single browser
npx playwright test --project=chromium

# All browsers
npx playwright test  # Runs Chromium, Firefox, WebKit
```

---

## 📊 What Gets Tested

| Category | Tests | Time |
|----------|-------|------|
| Critical Paths (smoke) | 16 | 5-10s |
| Components | 21 | 8-15s |
| Threads | 14 | 10-20s |
| Navigation | 9 | 6-10s |
| Streaming | 11 | 15-25s |
| Styleguide | 15 | 8-15s |
| Visual | 10+ | 5-10s |
| **Total** | **100+** | **60-100s** |

---

## ✅ Expected Output

```
100 tests | 100 passed | 0 failed (in ~90 seconds)
```

---

## 🐛 Troubleshooting

**Q: "npm: command not found"**  
A: Install Node.js from https://nodejs.org/

**Q: "Port 3000 already in use"**  
A: Kill existing process: `lsof -i :3000 | grep node | awk '{print $2}' | xargs kill -9`

**Q: Tests timeout**  
A: Run in debug mode: `npm run test:e2e:debug`

**Q: "Failed to fetch" errors**  
A: Tests expect backend at `http://localhost:3000`. Mock API or start backend.

---

## 📚 Full Guides

- **Setup Details:** [tests/e2e/SETUP.md](file:///home/ghuntley/loom/tests/e2e/SETUP.md)
- **Complete Guide:** [tests/e2e/README.md](file:///home/ghuntley/loom/tests/e2e/README.md)
- **CI Integration:** [tests/e2e/CI_INTEGRATION.md](file:///home/ghuntley/loom/tests/e2e/CI_INTEGRATION.md)
- **Full Report:** [E2E_TEST_EXECUTION_REPORT.md](file:///home/ghuntley/loom/E2E_TEST_EXECUTION_REPORT.md)

---

**Status:** ✅ Ready to run (install Node.js, then `npm install && npm run test:e2e`)
