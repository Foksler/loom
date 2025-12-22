# E2E Tests - CI/CD Integration

Integration guide for running Playwright E2E tests in CI/CD pipelines.

## GitHub Actions

### Simple Workflow

Create `.github/workflows/e2e-tests.yml`:

```yaml
name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  e2e:
    name: Playwright E2E
    runs-on: ubuntu-latest
    timeout-minutes: 10

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'
          cache: 'npm'

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup Leptos
        run: |
          rustup target add wasm32-unknown-unknown
          cargo install trunk
          cargo install cargo-leptos

      - name: Install dependencies (web)
        working-directory: crates/loom-web
        run: npm ci

      - name: Install Playwright browsers
        working-directory: crates/loom-web
        run: npx playwright install --with-deps chromium

      - name: Build loom-web
        run: cargo leptos build --release

      - name: Run E2E tests
        working-directory: crates/loom-web
        run: npm run test:e2e
        env:
          CI: true

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 30

      - name: Upload trace on failure
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-trace
          path: test-results/
          retention-days: 7
```

### Matrix Testing (Multiple Browsers)

```yaml
jobs:
  e2e:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        browser: [chromium, firefox, webkit]
    
    steps:
      - uses: actions/checkout@v4
      # ... setup steps ...
      
      - name: Install Playwright browsers
        working-directory: crates/loom-web
        run: npx playwright install --with-deps ${{ matrix.browser }}
      
      - name: Run E2E tests
        working-directory: crates/loom-web
        run: npm run test:e2e -- --project=${{ matrix.browser }}
        env:
          CI: true
```

### Scheduled Tests

Run tests nightly on a schedule:

```yaml
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily
  workflow_dispatch:     # Manual trigger

# ... rest of workflow ...
```

### Report Integration

#### Publish to GitHub Pages

```yaml
- name: Publish test report
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: github-pages
    path: playwright-report/

- name: Deploy to GitHub Pages
  if: always()
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./playwright-report
```

#### Post to PR

```yaml
- name: Comment on PR
  if: always()
  uses: actions/github-script@v6
  with:
    script: |
      const fs = require('fs');
      const testResults = JSON.parse(fs.readFileSync('test-results/results.json', 'utf8'));
      
      const comment = `
      ## E2E Test Results
      - Tests: ${testResults.stats.expected + testResults.stats.unexpected}
      - Passed: ${testResults.stats.expected}
      - Failed: ${testResults.stats.unexpected}
      - Skipped: ${testResults.stats.skipped}
      
      [View full report](https://github.com/${{ github.repository }}/actions/runs/${{ github.run_id }})
      `;
      
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: comment
      });
```

## GitLab CI

### Basic Configuration

Create `.gitlab-ci.yml`:

```yaml
e2e:test:
  image: mcr.microsoft.com/playwright:v1.40.0-focal
  stage: test
  
  cache:
    paths:
      - crates/loom-web/node_modules/
      - target/

  before_script:
    - rustup default stable
    - rustup target add wasm32-unknown-unknown
    - apt-get update && apt-get install -y cargo-leptos

  script:
    - cd crates/loom-web
    - npm ci
    - npm run test:e2e

  artifacts:
    when: always
    paths:
      - playwright-report/
      - test-results/
    reports:
      junit: playwright-report/junit.xml
    expire_in: 30 days

  allow_failure: true
```

### With Docker

```yaml
e2e:test:
  image: ghcr.io/ghuntley/loom:latest
  
  services:
    - name: localhost:5000/loom:test
      alias: app
  
  script:
    - cd crates/loom-web
    - npm ci
    - PLAYWRIGHT_TEST_BASE_URL=http://app:3000 npm run test:e2e
```

## Jenkins

### Jenkinsfile

```groovy
pipeline {
  agent any
  
  environment {
    NODE_ENV = 'test'
    CI = 'true'
  }

  stages {
    stage('Build') {
      steps {
        sh 'cargo build'
      }
    }

    stage('E2E Tests') {
      steps {
        dir('crates/loom-web') {
          sh 'npm ci'
          sh 'npx playwright install'
          sh 'npm run test:e2e'
        }
      }
    }
  }

  post {
    always {
      publishHTML([
        reportDir: 'playwright-report',
        reportFiles: 'index.html',
        reportName: 'E2E Test Report'
      ])
      
      junit 'test-results/**/*.xml'
    }

    failure {
      archiveArtifacts artifacts: 'test-results/**/*', allowEmptyArchive: true
    }
  }
}
```

## Makefile Integration

The Makefile already has E2E targets integrated:

```makefile
# Run E2E tests
test-e2e:
	@echo "Running E2E tests..."
	cd crates/loom-web && npm run test:e2e

# Full CI check
check: check-format lint build test test-e2e
	@echo "All checks passed!"
```

Run full CI locally:
```bash
make check
```

## Performance Optimization for CI

### Reduce Browser Count

For faster CI, test only Chromium:

Update `playwright.config.ts`:

```typescript
projects: process.env.CI ? [
  { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
] : [
  { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
  { name: 'webkit', use: { ...devices['Desktop Safari'] } },
],
```

### Parallel Workers

```typescript
workers: process.env.CI ? 4 : undefined,  // More workers in CI
```

### Reduce Artifacts

```typescript
use: {
  video: 'retain-on-failure',  // Not 'retain-all'
  trace: 'on-first-retry',     // Not 'on'
  screenshot: 'only-on-failure',
},
```

### Test Filtering

Run critical tests first, full suite in separate job:

```yaml
jobs:
  critical:
    runs-on: ubuntu-latest
    steps:
      # ... setup ...
      - run: npm run test:e2e -- --grep "Critical"

  full:
    runs-on: ubuntu-latest
    needs: critical
    steps:
      # ... setup ...
      - run: npm run test:e2e
```

## Artifact Management

### Upload Reports

GitHub Actions:
```yaml
- uses: actions/upload-artifact@v3
  with:
    name: e2e-report-${{ matrix.os }}-${{ matrix.browser }}
    path: playwright-report/
    retention-days: 30
```

### Download & View Reports

```bash
# GitHub CLI
gh run download <RUN_ID> -n e2e-report
npx playwright show-report

# Or access via GitHub Actions UI
```

## Notifications

### Slack Integration

```yaml
- name: Slack notification
  if: failure()
  uses: slackapi/slack-github-action@v1.24.0
  with:
    webhook-url: ${{ secrets.SLACK_WEBHOOK }}
    payload: |
      {
        "text": "E2E tests failed in ${{ github.repository }}",
        "blocks": [
          {
            "type": "section",
            "text": {
              "type": "mrkdwn",
              "text": "E2E Tests Failed\n*Repo:* ${{ github.repository }}\n*Branch:* ${{ github.ref }}\n*Run:* <${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}>"
            }
          }
        ]
      }
```

### Email Notifications

Configure in CI platform settings or use actions:

```yaml
- name: Send email on failure
  if: failure()
  uses: dawidd6/action-send-mail@v3
  with:
    server_address: ${{ secrets.EMAIL_SERVER }}
    server_port: 465
    username: ${{ secrets.EMAIL_USERNAME }}
    password: ${{ secrets.EMAIL_PASSWORD }}
    subject: 'E2E Tests Failed: ${{ github.repository }}'
    to: devteam@example.com
    from: ci@example.com
    body: |
      E2E tests failed!
      Repository: ${{ github.repository }}
      Branch: ${{ github.ref }}
      Run: ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}
```

## Environment Variables

Set in CI platform:

| Variable | Purpose | Example |
|----------|---------|---------|
| `CI` | Run in CI mode | `true` |
| `DEBUG` | Enable debug logging | `pw:api` |
| `PLAYWRIGHT_TEST_BASE_URL` | App URL | `http://localhost:3000` |
| `HEADED` | Run headed | `1` (only for debugging) |
| `TIMEOUT` | Test timeout | `30000` |

## Troubleshooting CI

### Browser Installation Fails

Add `--with-deps` flag:
```bash
npx playwright install --with-deps chromium
```

### Port Already in Use

```yaml
- name: Check port availability
  run: lsof -i :3000 || true

- name: Start server on different port
  env:
    PORT: 3001
  run: cargo leptos watch &
```

### Timeout Issues

Increase timeout in CI:

```yaml
- name: Run E2E tests
  env:
    PLAYWRIGHT_TIMEOUT: 60000  # 60s
  run: npm run test:e2e
```

Or in config:
```typescript
use: {
  navigationTimeout: 60000,
  actionTimeout: 30000,
}
```

### Memory Issues

```yaml
- name: Run E2E tests
  env:
    NODE_OPTIONS: --max-old-space-size=2048
  run: npm run test:e2e -- --workers=1
```

## Caching

### Cache Dependencies

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: '18'
    cache: 'npm'
    cache-dependency-path: 'crates/loom-web/package-lock.json'

- uses: Swatinem/rust-cache@v2
```

### Cache Playwright Browsers

```yaml
- uses: actions/cache@v3
  id: playwright-cache
  with:
    path: ~/.cache/ms-playwright
    key: ${{ runner.os }}-playwright-${{ hashFiles('**/package-lock.json') }}

- run: npx playwright install
  if: steps.playwright-cache.outputs.cache-hit != 'true'
```

## Best Practices

✅ **Do**:
- Run critical tests first
- Use matrix testing for browsers
- Cache dependencies
- Upload reports and traces
- Set timeouts generously
- Filter network errors

❌ **Don't**:
- Test on every single commit (use scheduled jobs)
- Leave `.only()` in tests
- Run full matrix for every change
- Download all browsers (use chromium only)
- Store large artifacts indefinitely

## See Also

- [GitHub Actions Best Practices](https://docs.github.com/en/actions/guides)
- [GitLab CI/CD Documentation](https://docs.gitlab.com/ee/ci/)
- [Playwright CI Guide](https://playwright.dev/docs/ci)
