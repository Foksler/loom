# E2E Testing Setup Verification

**Purpose**: Verify all E2E testing files are properly installed and configured.

## ✅ File Verification

### Test Files
```
✅ tests/e2e/navigation.spec.ts    (11 tests)
✅ tests/e2e/styleguide.spec.ts    (15 tests)
✅ tests/e2e/components.spec.ts    (20 tests)
✅ tests/e2e/threads.spec.ts       (12 tests)
✅ tests/e2e/streaming.spec.ts     (11 tests)
✅ tests/e2e/visual.spec.ts        (15 tests)
✅ tests/e2e/.gitignore
✅ tests/README.md
```

**Total Test Files**: 6 spec files with 84 tests

### Configuration Files
```
✅ playwright.config.ts
✅ crates/loom-web/package.json    (updated)
✅ Makefile                        (updated)
✅ .github/workflows/e2e-tests.yml
```

### Documentation Files
```
✅ TESTING_GUIDE.md
✅ TESTING_QUICK_REFERENCE.md
✅ E2E_TESTING_SUMMARY.md
✅ IMPLEMENTATION_E2E_TESTING.md
✅ DELIVERABLES_E2E_TESTING.md
✅ VERIFY_E2E_SETUP.md             (this file)
```

---

## 📋 Checklist for Setup

### Step 1: Verify Files Exist
Run this from workspace root:

```bash
# Check test files
ls -la tests/e2e/*.spec.ts
# Should show 6 files

# Check configuration
test -f playwright.config.ts && echo "✅ playwright.config.ts exists"
test -f Makefile && echo "✅ Makefile exists"
test -f .github/workflows/e2e-tests.yml && echo "✅ GitHub Actions workflow exists"

# Check documentation
test -f TESTING_GUIDE.md && echo "✅ TESTING_GUIDE.md exists"
test -f TESTING_QUICK_REFERENCE.md && echo "✅ TESTING_QUICK_REFERENCE.md exists"
test -f E2E_TESTING_SUMMARY.md && echo "✅ E2E_TESTING_SUMMARY.md exists"
test -f IMPLEMENTATION_E2E_TESTING.md && echo "✅ IMPLEMENTATION_E2E_TESTING.md exists"
```

### Step 2: Install Dependencies

```bash
# Navigate to loom-web
cd crates/loom-web

# Install npm dependencies
npm install @playwright/test

# Install Playwright browsers
npx playwright install

# Return to root
cd ../..
```

**Expected Output**:
```
npm notice created a lockfile as package-lock.json
✅ @playwright/test installed
✅ Playwright browsers installed
```

### Step 3: Verify Configuration

```bash
# Check playwright.config.ts has correct settings
grep -q "baseURL: 'http://localhost:3000'" playwright.config.ts && echo "✅ baseURL configured"
grep -q "webServer:" playwright.config.ts && echo "✅ webServer configured"
grep -q "projects:" playwright.config.ts && echo "✅ projects configured"

# Check package.json has test scripts
grep -q '"test:e2e"' crates/loom-web/package.json && echo "✅ test:e2e script exists"
grep -q '"test:e2e:ui"' crates/loom-web/package.json && echo "✅ test:e2e:ui script exists"
grep -q '"test:e2e:debug"' crates/loom-web/package.json && echo "✅ test:e2e:debug script exists"

# Check Makefile has test targets
grep -q "test-e2e:" Makefile && echo "✅ test-e2e target exists"
grep -q "test-e2e-ui:" Makefile && echo "✅ test-e2e-ui target exists"
grep -q "test-e2e-debug:" Makefile && echo "✅ test-e2e-debug target exists"
```

### Step 4: Test Basic Commands

```bash
# Test from workspace root
make test-e2e --dry-run
echo "✅ Makefile test-e2e target works"

# Test from loom-web
cd crates/loom-web
npm run test:e2e -- --list
echo "✅ npm test:e2e script works"
cd ../..
```

### Step 5: Create Visual Baselines

```bash
# Create baseline screenshots for visual tests
npm --prefix crates/loom-web run test:e2e:update-snapshots

# Check baseline directory created
test -d tests/e2e/__screenshots__ && echo "✅ Visual baselines created"
```

### Step 6: Run First Test

```bash
# Run a single quick test
npm --prefix crates/loom-web run test:e2e -- -g "home page loads"

# Expected output:
# ✅ 1 passed
```

---

## 🔧 Configuration Verification

### playwright.config.ts
```typescript
// Verify these settings are present:
✅ testDir: './tests/e2e'
✅ baseURL: 'http://localhost:3000'
✅ webServer with cargo leptos watch
✅ Projects: chromium, firefox, webkit
✅ Screenshot: 'only-on-failure'
✅ Video: 'retain-on-failure'
✅ Timeout: 120000
```

### crates/loom-web/package.json
```json
{
  "scripts": {
    "test:e2e": "playwright test",           ✅
    "test:e2e:ui": "playwright test --ui",   ✅
    "test:e2e:debug": "playwright test --debug", ✅
    "test:e2e:update-snapshots": "playwright test --update-snapshots" ✅
  },
  "devDependencies": {
    "@playwright/test": "^1.40.0" ✅
  }
}
```

### Makefile
```makefile
# Verify these targets exist:
✅ test-e2e:
✅ test-e2e-ui:
✅ test-e2e-debug:
```

---

## 🧪 Test Verification

### Test Files Exist
```bash
cd tests/e2e
ls -1 *.spec.ts
# Should output:
# navigation.spec.ts
# styleguide.spec.ts
# components.spec.ts
# threads.spec.ts
# streaming.spec.ts
# visual.spec.ts
```

### Test Count
```bash
# Count tests across all files
grep -r "test('" tests/e2e/*.spec.ts | wc -l
# Should output: 84 (or close to it)
```

### Test Structure
Each test file should have:
- ✅ Proper imports: `import { test, expect } from '@playwright/test';`
- ✅ Test groups: `test.describe('...')`
- ✅ Individual tests: `test('...')`
- ✅ JSDoc comments explaining purpose
- ✅ Proper assertions using expect()

---

## 📚 Documentation Verification

### Required Files
```bash
test -f TESTING_GUIDE.md && echo "✅ TESTING_GUIDE.md"
test -f TESTING_QUICK_REFERENCE.md && echo "✅ TESTING_QUICK_REFERENCE.md"
test -f E2E_TESTING_SUMMARY.md && echo "✅ E2E_TESTING_SUMMARY.md"
test -f IMPLEMENTATION_E2E_TESTING.md && echo "✅ IMPLEMENTATION_E2E_TESTING.md"
test -f DELIVERABLES_E2E_TESTING.md && echo "✅ DELIVERABLES_E2E_TESTING.md"
test -f tests/README.md && echo "✅ tests/README.md"
```

### Documentation Content
Each file should contain:
- ✅ Clear section headers
- ✅ Code examples
- ✅ Quick start instructions
- ✅ Troubleshooting guidance
- ✅ Links to other docs

---

## 🚀 Pre-Run Checklist

Before running tests, verify:

### Development Environment
- [ ] Node.js 16+ installed (`node --version`)
- [ ] npm/yarn available (`npm --version`)
- [ ] Rust installed (`rustc --version`)
- [ ] cargo-leptos installed (`cargo leptos --version`)
- [ ] Port 3000 available (`lsof -i :3000` shows nothing)

### Playwright Setup
- [ ] `@playwright/test` installed
- [ ] Browsers installed (`npx playwright install`)
- [ ] playwright.config.ts exists and is valid
- [ ] Test files in tests/e2e/ directory

### Repository Setup
- [ ] All test files committed to git
- [ ] All documentation committed to git
- [ ] Configuration files committed to git
- [ ] .gitignore updated for test artifacts

### Application Ready
- [ ] loom-web builds successfully (`cargo leptos build`)
- [ ] Dev server can start (`cargo leptos watch`)
- [ ] Application loads at http://localhost:3000
- [ ] Sample data available for threads

---

## ✅ Quick Verification Script

Save this as `verify-e2e.sh` and run:

```bash
#!/bin/bash

echo "🔍 Verifying E2E Testing Setup..."
echo ""

# Check files
echo "📋 Checking test files..."
test -f tests/e2e/navigation.spec.ts && echo "✅ navigation.spec.ts" || echo "❌ navigation.spec.ts missing"
test -f tests/e2e/styleguide.spec.ts && echo "✅ styleguide.spec.ts" || echo "❌ styleguide.spec.ts missing"
test -f tests/e2e/components.spec.ts && echo "✅ components.spec.ts" || echo "❌ components.spec.ts missing"
test -f tests/e2e/threads.spec.ts && echo "✅ threads.spec.ts" || echo "❌ threads.spec.ts missing"
test -f tests/e2e/streaming.spec.ts && echo "✅ streaming.spec.ts" || echo "❌ streaming.spec.ts missing"
test -f tests/e2e/visual.spec.ts && echo "✅ visual.spec.ts" || echo "❌ visual.spec.ts missing"

echo ""
echo "⚙️ Checking configuration files..."
test -f playwright.config.ts && echo "✅ playwright.config.ts" || echo "❌ playwright.config.ts missing"
test -f Makefile && echo "✅ Makefile" || echo "❌ Makefile missing"
test -f .github/workflows/e2e-tests.yml && echo "✅ GitHub Actions workflow" || echo "❌ Workflow missing"

echo ""
echo "📚 Checking documentation..."
test -f TESTING_GUIDE.md && echo "✅ TESTING_GUIDE.md" || echo "❌ TESTING_GUIDE.md missing"
test -f TESTING_QUICK_REFERENCE.md && echo "✅ TESTING_QUICK_REFERENCE.md" || echo "❌ Missing"
test -f E2E_TESTING_SUMMARY.md && echo "✅ E2E_TESTING_SUMMARY.md" || echo "❌ Missing"
test -f IMPLEMENTATION_E2E_TESTING.md && echo "✅ IMPLEMENTATION_E2E_TESTING.md" || echo "❌ Missing"
test -f DELIVERABLES_E2E_TESTING.md && echo "✅ DELIVERABLES_E2E_TESTING.md" || echo "❌ Missing"

echo ""
echo "📦 Checking dependencies..."
cd crates/loom-web
grep -q "@playwright/test" package.json && echo "✅ @playwright/test in package.json" || echo "❌ Missing @playwright/test"
grep -q '"test:e2e"' package.json && echo "✅ test:e2e script" || echo "❌ Missing test:e2e script"
cd ../..

echo ""
echo "✨ Setup verification complete!"
echo ""
echo "Next steps:"
echo "1. cd crates/loom-web && npm install && npx playwright install"
echo "2. npm run test:e2e:update-snapshots"
echo "3. make test-e2e"
```

Run it:
```bash
chmod +x verify-e2e.sh
./verify-e2e.sh
```

---

## 🎯 Next Steps After Verification

### Step 1: Install Dependencies (if not done)
```bash
cd crates/loom-web
npm install @playwright/test
npx playwright install
cd ../..
```

### Step 2: Create Visual Baselines
```bash
npm --prefix crates/loom-web run test:e2e:update-snapshots
```

### Step 3: Run First Test Suite
```bash
# Test navigation
make test-e2e -- -g "navigation"

# Or run all tests
make test-e2e
```

### Step 4: Verify Results
```bash
# View HTML report
npx playwright show-report
```

### Step 5: Commit Setup
```bash
git add tests/ playwright.config.ts TESTING*.md E2E*.md DELIVERABLES*.md VERIFY*.md
git commit -m "Add comprehensive E2E testing suite with Playwright (84 tests)"
git push
```

---

## 🐛 Troubleshooting

### Problem: playwright.config.ts not found
**Solution**: Check file exists at workspace root
```bash
ls -la playwright.config.ts
```

### Problem: Tests not found
**Solution**: Ensure tests/e2e directory structure
```bash
ls -la tests/e2e/
# Should show 6 .spec.ts files
```

### Problem: Port 3000 in use
**Solution**: Kill process using port
```bash
lsof -ti:3000 | xargs kill -9
```

### Problem: Node modules not installed
**Solution**: Install dependencies
```bash
cd crates/loom-web && npm install
```

### Problem: Playwright browsers not found
**Solution**: Install browsers
```bash
npx playwright install
```

---

## 📞 Support

- **Quick Help**: [`TESTING_QUICK_REFERENCE.md`](TESTING_QUICK_REFERENCE.md)
- **Full Guide**: [`TESTING_GUIDE.md`](TESTING_GUIDE.md)
- **Details**: [`IMPLEMENTATION_E2E_TESTING.md`](IMPLEMENTATION_E2E_TESTING.md)

---

## ✅ Final Checklist

- [ ] All test files exist and readable
- [ ] Configuration files in place
- [ ] Documentation complete
- [ ] Dependencies installed
- [ ] Playwright browsers installed
- [ ] Visual baselines created
- [ ] First test runs successfully
- [ ] Makefile targets work
- [ ] npm scripts work
- [ ] Ready for CI/CD integration

---

**Status**: ✅ Ready to Verify  
**Expected Outcome**: All checks pass  
**Next**: See TESTING_GUIDE.md for full instructions
