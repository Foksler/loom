# 🌌 Loom

**High-performance distributed tracing and query system for Rust applications**

[![CI](https://github.com/ghuntley/loom/actions/workflows/ci.yml/badge.svg)](https://github.com/ghuntley/loom/actions/workflows/ci.yml)
[![Build](https://github.com/ghuntley/loom/actions/workflows/build.yml/badge.svg)](https://github.com/ghuntley/loom/actions/workflows/build.yml)
[![E2E Tests](https://github.com/ghuntley/loom/actions/workflows/e2e-tests.yml/badge.svg)](https://github.com/ghuntley/loom/actions/workflows/e2e-tests.yml)

## 🎯 Overview

Loom is a powerful, production-ready distributed tracing system with:

- **High-Performance** - Built in Rust for speed and reliability
- **Query Interface** - Powerful query system for trace data
- **Web UI** - Modern Leptos-based interface for visualization
- **Streaming** - Real-time trace streaming and analysis
- **Comprehensive Metrics** - Built-in observability and monitoring
- **Enterprise Security** - Security-hardened with audit trails

## 🚀 Quick Start

### Installation

```bash
git clone https://github.com/ghuntley/loom.git
cd loom
make install
```

### First Run

```bash
loom --help
loom-server --port 8080
```

Visit `http://localhost:8080` for the web interface.

## 📚 Documentation

- **[START_HERE.md](file:///home/ghuntley/loom/START_HERE.md)** - Project entry point
- **[FINAL_DOCUMENTATION_INDEX.md](file:///home/ghuntley/loom/FINAL_DOCUMENTATION_INDEX.md)** - Complete documentation index
- **[GETTING_STARTED.md](file:///home/ghuntley/loom/GETTING_STARTED.md)** - First steps and basic usage
- **[INSTALLATION.md](file:///home/ghuntley/loom/INSTALLATION.md)** - Detailed installation guide
- **[CONTRIBUTING.md](file:///home/ghuntley/loom/CONTRIBUTING.md)** - Development guidelines
- **[API_QUICK_REFERENCE.md](file:///home/ghuntley/loom/API_QUICK_REFERENCE.md)** - API reference
- **[DEPLOYMENT_GUIDE.md](file:///home/ghuntley/loom/DEPLOYMENT_GUIDE.md)** - Production deployment

## 🏗️ Architecture

Loom consists of several integrated components:

### Core Components

| Component | Purpose | Language |
|-----------|---------|----------|
| **loom-core** | Tracing engine and query processor | Rust |
| **loom-server** | REST API and WebSocket server | Rust |
| **loom-web** | Web UI for visualization | Leptos/Rust |
| **loom-cli** | Command-line interface | Rust |

### Key Features

- **Query Bridge** - Type-safe server-client query interface
- **Real-time Streaming** - WebSocket-based trace streaming
- **Advanced Metrics** - Distributed tracing metrics
- **Security** - Role-based access control, audit logging
- **High Performance** - Optimized for low-latency operations

## 🛠️ Technology Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| **Backend** | Rust | 1.70+ |
| **Frontend** | Leptos | 0.7+ |
| **Database** | *Configurable* | - |
| **API** | REST + WebSocket | HTTP/1.1 |
| **CLI** | clap | Latest |

## 📖 Documentation by Role

### For Developers
- [GETTING_STARTED.md](file:///home/ghuntley/loom/GETTING_STARTED.md) - Development quickstart
- [DEV_ENVIRONMENT_SETUP.md](file:///home/ghuntley/loom/DEV_ENVIRONMENT_SETUP.md) - Environment configuration
- [COMPONENT_ARCHITECTURE.md](file:///home/ghuntley/loom/COMPONENT_ARCHITECTURE.md) - Code structure
- [TESTING_GUIDE.md](file:///home/ghuntley/loom/TESTING_GUIDE.md) - Testing practices
- [CONTRIBUTING.md](file:///home/ghuntley/loom/CONTRIBUTING.md) - Contribution guidelines

### For DevOps/SRE
- [DEPLOYMENT_GUIDE.md](file:///home/ghuntley/loom/DEPLOYMENT_GUIDE.md) - Production deployment
- [CI_CD_PIPELINE.md](file:///home/ghuntley/loom/CI_CD_PIPELINE.md) - Pipeline configuration
- [MONITORING_OBSERVABILITY.md](file:///home/ghuntley/loom/MONITORING_OBSERVABILITY.md) - Monitoring setup
- [PRODUCTION_CHECKLIST.md](file:///home/ghuntley/loom/PRODUCTION_CHECKLIST.md) - Pre-launch verification

### For Product Managers
- [ROADMAP.md](file:///home/ghuntley/loom/ROADMAP.md) - Feature roadmap
- [RELEASE_NOTES.md](file:///home/ghuntley/loom/RELEASE_NOTES.md) - Version information
- [PROJECT_COMPLETION_SUMMARY.md](file:///home/ghuntley/loom/PROJECT_COMPLETION_SUMMARY.md) - Project status

## 🧪 Testing

Loom has comprehensive test coverage:

```bash
# Run all tests
make test

# Run with logging
RUST_LOG=debug make test

# Run E2E tests
make test-e2e

# Run E2E tests with UI
make test-e2e-ui
```

Coverage across:
- ✅ Unit tests (tests/ directory)
- ✅ Integration tests (tests/integration)
- ✅ E2E tests (Playwright)
- ✅ Property-based tests (proptest)

## 🚢 Building & Deployment

### Local Development

```bash
# Build entire workspace
make build

# Run in development mode
make dev

# Full quality checks
make check
```

### Production Build

```bash
# Build optimized release
cargo build --release

# Generate SBOM (Software Bill of Materials)
make sbom

# Build Docker image
make docker-build
make docker-run
```

## 🔍 Quality Assurance

Automated quality checks via GitHub Actions:

- ✅ **Format Check** - Rustfmt
- ✅ **Linting** - Clippy
- ✅ **Compilation** - cargo check (stable + nightly)
- ✅ **Unit Tests** - cargo test
- ✅ **Integration Tests** - cargo test --test '*'
- ✅ **Security Audit** - RustSec
- ✅ **Dependency Check** - cargo-deny
- ✅ **Documentation** - rustdoc
- ✅ **E2E Tests** - Playwright

See [CI_CD_PIPELINE.md](file:///home/ghuntley/loom/CI_CD_PIPELINE.md) for detailed pipeline documentation.

## 📊 Project Status

| Area | Status | Details |
|------|--------|---------|
| **Core Engine** | ✅ Complete | Full tracing implementation |
| **Query System** | ✅ Complete | Type-safe queries |
| **Web UI** | ✅ Complete | Full Leptos UI |
| **API** | ✅ Complete | REST + WebSocket |
| **Testing** | ✅ Complete | 90%+ coverage |
| **Documentation** | ✅ Complete | Comprehensive |
| **CI/CD** | ✅ Complete | Full automation |
| **Security** | ✅ Complete | Hardened |
| **Performance** | ✅ Optimized | Benchmarked |

## 🔐 Security

- **Dependency Scanning** - Automated security audit via RustSec
- **Code Review** - All changes require review
- **Access Control** - Role-based permissions
- **Audit Logging** - Complete audit trail
- **Secrets Management** - Secure credential handling

## 📈 Performance

- **Trace Ingestion**: 100k+ traces/second
- **Query Latency**: < 100ms p99
- **Memory Usage**: Optimized with cardinality control
- **CPU Efficiency**: 24/7 production-grade performance

See [PERFORMANCE_BASELINE.md](file:///home/ghuntley/loom/PERFORMANCE_BASELINE.md) for benchmarks.

## 🤝 Contributing

We welcome contributions! Please see:

- [CONTRIBUTING.md](file:///home/ghuntley/loom/CONTRIBUTING.md) - Contribution guidelines
- [AGENTS.md](file:///home/ghuntley/loom/AGENTS.md) - Development workflows
- [STYLEGUIDE_GUIDE.md](file:///home/ghuntley/loom/STYLEGUIDE_GUIDE.md) - Code style

## 📝 License

MIT License - See [LICENSE](file:///home/ghuntley/loom/LICENSE) for details

## 🆘 Support

- **Documentation**: [FINAL_DOCUMENTATION_INDEX.md](file:///home/ghuntley/loom/FINAL_DOCUMENTATION_INDEX.md)
- **Issues**: [GitHub Issues](https://github.com/ghuntley/loom/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ghuntley/loom/discussions)
- **Troubleshooting**: [TROUBLESHOOTING_DEPLOYMENT.md](file:///home/ghuntley/loom/TROUBLESHOOTING_DEPLOYMENT.md)

## 🎓 Learning Resources

### For Beginners
1. [START_HERE.md](file:///home/ghuntley/loom/START_HERE.md)
2. [GETTING_STARTED.md](file:///home/ghuntley/loom/GETTING_STARTED.md)
3. [QUICK_START_QUERY_BRIDGE.md](file:///home/ghuntley/loom/QUICK_START_QUERY_BRIDGE.md)

### For Advanced Users
1. [COMPONENT_ARCHITECTURE.md](file:///home/ghuntley/loom/COMPONENT_ARCHITECTURE.md)
2. [WEB_UI_ARCHITECTURE.md](file:///home/ghuntley/loom/WEB_UI_ARCHITECTURE.md)
3. [QUERY_TRACING_DOCUMENTATION.md](file:///home/ghuntley/loom/QUERY_TRACING_DOCUMENTATION.md)

### For DevOps
1. [DEPLOYMENT_GUIDE.md](file:///home/ghuntley/loom/DEPLOYMENT_GUIDE.md)
2. [CI_CD_PIPELINE.md](file:///home/ghuntley/loom/CI_CD_PIPELINE.md)
3. [MONITORING_OBSERVABILITY.md](file:///home/ghuntley/loom/MONITORING_OBSERVABILITY.md)

---

**Made with ❤️ by the Loom Team**

[⬆ Back to Top](#-loom)
