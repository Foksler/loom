# Development Documentation Index

Complete index of all development documentation and guides for the Loom project.

## Quick Navigation

### 🚀 Getting Started

**Start Here**: [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
- System requirements
- 3 installation methods
- IDE configuration
- Troubleshooting

**For Nix Users**: [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md)
- Nix flakes setup
- devenv configuration
- direnv auto-loading
- Performance tips

**Full Summary**: [DEV_ENVIRONMENT_COMPLETE.md](DEV_ENVIRONMENT_COMPLETE.md)
- Overview of all setup files
- Quick reference commands
- Workflow guide

### 👥 Contributing

**Contribution Guide**: [CONTRIBUTING.md](CONTRIBUTING.md)
- Code of conduct
- Development workflow
- Coding standards (Rust, TypeScript)
- Commit message format
- Pull request process
- Testing requirements
- Code review process

### 🔧 CI/CD & Automation

**CI/CD Pipeline**: [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md)
- Pipeline overview
- 12 job descriptions
- Local development matching
- Debugging guidelines
- Configuration reference

**GitHub Actions**: [.github/workflows/ci.yml](.github/workflows/ci.yml)
- Automated testing
- Security audits
- Build validation
- Documentation checks

### 🐳 Docker & Containerization

**Docker Compose**: [docker-compose.yml](docker-compose.yml)
- Web UI service
- Optional server service
- Database configuration
- Network setup

**Leptos Build**: [docker/Dockerfile.web](docker/Dockerfile.web)
- Multi-stage build
- Production optimization
- Health checks

**Web Server**: [docker/nginx.conf](docker/nginx.conf)
- Reverse proxy configuration
- SPA routing
- Security headers
- Caching strategy

### 📝 Development Commands

**Makefile**: [Makefile](Makefile)

Core commands:
```bash
make help              # Show all targets
make build             # Build workspace
make test              # Run tests
make lint              # Linting
make format            # Code formatting
make check             # Full CI locally
make dev               # Watch mode
make docker-build      # Docker build
make test-e2e          # E2E tests
```

## Documentation by Topic

### Setup & Installation

| Document | Purpose | Audience |
|----------|---------|----------|
| [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md) | Main setup guide for all platforms | Everyone |
| [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md) | Nix/devenv specific setup | Nix users |
| [DEV_ENVIRONMENT_COMPLETE.md](DEV_ENVIRONMENT_COMPLETE.md) | Summary and quick reference | Everyone |
| [shell.nix](shell.nix) | Traditional Nix shell | Nix users |

### Development Workflow

| Document | Purpose | Audience |
|----------|---------|----------|
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute code | Contributors |
| [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md) | Understanding CI/CD checks | All developers |
| [Makefile](Makefile) | Development commands | All developers |

### Deployment & Containerization

| Document | Purpose | Audience |
|----------|---------|----------|
| [docker-compose.yml](docker-compose.yml) | Docker setup | DevOps, developers |
| [docker/Dockerfile.web](docker/Dockerfile.web) | Web UI container | DevOps, developers |
| [docker/nginx.conf](docker/nginx.conf) | Web server config | DevOps, developers |

### Configuration

| File | Purpose |
|------|---------|
| [.github/workflows/ci.yml](.github/workflows/ci.yml) | GitHub Actions pipeline |
| [Makefile](Makefile) | Development commands |
| [shell.nix](shell.nix) | Nix shell environment |
| [docker-compose.yml](docker-compose.yml) | Docker orchestration |

## Common Tasks

### Setting Up Development Environment

1. Read: [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
2. Choose installation method
3. Follow setup steps for your OS
4. Verify with: `make check`

### Starting Development

```bash
# View available commands
make help

# Use watch mode
make dev

# Or use your IDE
# See IDE configuration in DEV_ENVIRONMENT_SETUP.md
```

### Before Committing Code

1. Read: [CONTRIBUTING.md](CONTRIBUTING.md) - Coding standards
2. Format code: `make format`
3. Check linting: `make lint`
4. Run tests: `make test`
5. Full check: `make check`
6. Commit with proper message format (see Contributing guide)

### Creating a Pull Request

1. See [CONTRIBUTING.md](CONTRIBUTING.md) - Pull Request Process
2. Push code to your fork
3. Create PR on GitHub
4. CI automatically runs (see [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md))
5. Wait for approval and merge

### Debugging CI Failures

1. See [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md) - Monitoring & Debugging
2. Run same checks locally
3. Use `cargo` directly for more details
4. Check logs in GitHub Actions UI

### Setting Up Docker

```bash
# Quick start
make docker-build
make docker-run

# Or with Docker Compose
docker-compose up --build
```

See [docker-compose.yml](docker-compose.yml) for configuration.

### Using Nix for Development

1. Read: [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md)
2. Run: `direnv allow` or `nix develop`
3. Development environment auto-loads
4. Use normally with Makefile

## File Structure

```
loom/
├── .github/
│   └── workflows/
│       └── ci.yml                     # GitHub Actions
├── docker/
│   ├── Dockerfile.web                 # Web UI build
│   └── nginx.conf                     # Web server
├── Makefile                           # Development commands
├── shell.nix                          # Nix shell
├── flake.nix                          # Nix flakes
├── devenv.nix                         # devenv config
├── docker-compose.yml                 # Docker Compose
│
├── DEV_ENVIRONMENT_SETUP.md           # START HERE
├── DEV_ENVIRONMENT_NIX.md             # For Nix users
├── DEV_ENVIRONMENT_COMPLETE.md        # Full summary
├── CI_CD_PIPELINE.md                  # CI/CD docs
├── CONTRIBUTING.md                    # Contributor guide
└── DEV_DOCUMENTATION_INDEX.md         # This file
```

## Reading Guide

### I'm New to Loom

1. Start: [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
2. Setup: Choose and follow your OS installation
3. Check: Run `make check` to verify
4. Learn: Read [DEV_ENVIRONMENT_COMPLETE.md](DEV_ENVIRONMENT_COMPLETE.md)

### I Want to Contribute Code

1. Setup: Follow [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
2. Standards: Read [CONTRIBUTING.md](CONTRIBUTING.md)
3. Workflow: Follow development workflow in Contributing guide
4. Test: Run `make check` before pushing
5. PR: Create pull request and follow review process

### I'm Debugging a CI Failure

1. Check: [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md) - "Troubleshooting" section
2. Local: Run same checks locally with `make` targets
3. Logs: View GitHub Actions logs in PR checks tab
4. Fix: Address issues and push again

### I Want to Use Docker

1. Setup: Read [docker-compose.yml](docker-compose.yml)
2. Build: Run `make docker-build`
3. Run: Run `make docker-run` or `docker-compose up`
4. Config: Customize [docker/nginx.conf](docker/nginx.conf) if needed

### I Use Nix

1. Setup: [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md)
2. direnv: Run `direnv allow` for auto-loading
3. Shell: Or use `nix develop` manually
4. Dev: Use normally - all tools provided

### I Use an IDE

1. Setup: Read [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md) - "IDE Configuration"
2. VS Code: Copy provided settings
3. Vim: Install rust-analyzer
4. JetBrains: Install Rust plugin
5. Dev: Use IDE normally - Makefile for batch commands

## Quick Reference

### Essential Commands

```bash
# Setup
direnv allow          # Auto-load environment (Nix)
nix develop          # Manual environment (Nix)

# Development
make dev             # Watch mode
make build           # Build
make test            # Tests
make lint            # Linting
make format          # Code format

# Quality
make check           # Full CI locally (before push!)
make fix             # Auto-fix issues

# Docker
make docker-build    # Build image
make docker-run      # Run container

# Help
make help            # Show all targets
```

### Commit Message Format

```
type(scope): description

type: feat, fix, docs, style, refactor, perf, test, chore, ci
scope: loom-core, loom-server, loom-web, llm, etc.
description: imperative, max 50 chars
```

Example:
```
feat(loom-core): add streaming query support

Implement streaming responses for long-running queries.

Fixes #456
```

### Code Review Checklist

From [CONTRIBUTING.md](CONTRIBUTING.md):
- [ ] Code is clear and documented
- [ ] Tests cover new functionality
- [ ] No unnecessary dependencies
- [ ] Performance not degraded
- [ ] Error handling appropriate
- [ ] Security considered
- [ ] Follows style guide

## Troubleshooting

### Most Common Issues

| Problem | Solution |
|---------|----------|
| Build fails | See [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md) - "Common Issues" |
| Tests fail | See [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md) - "Common CI Failures" |
| CI failure | See [CI_CD_PIPELINE.md](CI_CD_PIPELINE.md) - "Debugging" |
| Nix issues | See [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md) - "Troubleshooting" |

## Support & Help

- **Issues**: https://github.com/ghuntley/loom/issues
- **Discussions**: https://github.com/ghuntley/loom/discussions
- **Code examples**: `examples/` directory
- **API docs**: See `API_QUICK_REFERENCE.md`

## FAQ

**Q: Which setup method should I use?**
A: If you have Nix, use it (recommended). Otherwise, quick start or manual setup.

**Q: Do I need Docker?**
A: No, optional. Use for containerized development or deployment.

**Q: How do I run tests locally?**
A: `make test` or `cargo test --workspace`

**Q: What's the commit message format?**
A: See [CONTRIBUTING.md](CONTRIBUTING.md) - Conventional Commits format

**Q: How often should I run `make check`?**
A: Before every commit - it catches CI issues early

**Q: Can I skip CI checks?**
A: Not recommended. CI prevents bugs. Use `[skip ci]` in commit message only for docs.

**Q: Where do I ask questions?**
A: GitHub Issues or Discussions - see links above

## Last Updated

December 2024

---

**Start with**: [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
