# 📦 Loom Installation Guide

Complete step-by-step installation instructions for all platforms.

## 🚀 Quick Install

### macOS (Homebrew)
```bash
brew install ghuntley/loom/loom
loom --version
```

### Linux (APT)
```bash
sudo apt-add-repository ppa:ghuntley/loom
sudo apt update
sudo apt install loom
loom --version
```

### Windows (Chocolatey)
```powershell
choco install loom
loom --version
```

### Universal (Cargo)
```bash
cargo install loom-cli
loom --version
```

---

## 📋 System Requirements

### Minimum Requirements
- **CPU**: 2+ cores
- **RAM**: 2 GB minimum (4 GB recommended)
- **Disk**: 100 MB free
- **OS**: Linux, macOS, Windows, BSD

### Recommended Setup
- **CPU**: 4+ cores
- **RAM**: 8+ GB
- **Disk**: 1 GB+ free
- **Network**: 100 Mbps+

### Development Requirements
- **Rust**: 1.70 or newer
- **Node.js**: 18 or newer (for web UI)
- **Docker**: Optional (for containerized deployment)

---

## 🔧 Installation Methods

### Method 1: Clone & Build (Development)

**Prerequisites**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

**Clone & Build**:
```bash
# Clone repository
git clone https://github.com/ghuntley/loom.git
cd loom

# Build entire workspace
make build

# Verify build
./target/debug/loom --version

# Install to PATH
make install
```

**Time Required**: 5-15 minutes (depends on internet speed)

### Method 2: Release Binaries (Production)

**Download**:
```bash
# Find latest release
curl -s https://api.github.com/repos/ghuntley/loom/releases/latest | grep browser_download_url

# Download (Linux example)
wget https://github.com/ghuntley/loom/releases/download/v1.0.0/loom-linux-x86_64

# Make executable
chmod +x loom-linux-x86_64

# Install to PATH
sudo mv loom-linux-x86_64 /usr/local/bin/loom
```

### Method 3: Docker (Containerized)

**Using Pre-built Image**:
```bash
# Pull image
docker pull ghuntley/loom:latest

# Run container
docker run -d \
  --name loom-server \
  -p 8080:8080 \
  ghuntley/loom:latest

# Check logs
docker logs loom-server

# Access web UI
open http://localhost:8080
```

**Building Image Locally**:
```bash
# Build via Nix
make docker-build

# Load into Docker
docker load < ./result

# Run
make docker-run
```

### Method 4: Development Environment (Nix)

**Using Nix Flakes**:
```bash
# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# Enter dev environment
nix flake update
nix develop

# Now you have all dev tools
cargo build
npm run build
```

---

## ✅ Verification

### Basic Installation Check

```bash
# Check CLI
loom --version
loom --help

# Check server binary
loom-server --help

# Verify all binaries
which loom
which loom-server
```

### Full Diagnostic

```bash
# Run diagnostics
loom doctor

# Check Rust setup
rustc --version
cargo --version

# Check Node.js (for web UI)
node --version
npm --version
```

---

## 🔌 System Configuration

### Environment Variables

**Optional Settings**:
```bash
# Logging level
export RUST_LOG=debug

# Thread pool size
export RAYON_NUM_THREADS=4

# Trace buffer size
export LOOM_BUFFER_SIZE=10000
```

### Linux System Configuration

**Increase file descriptors** (for high-volume tracing):
```bash
# Check current limit
ulimit -n

# Increase temporarily
ulimit -n 65536

# Increase permanently in ~/.bashrc
echo 'ulimit -n 65536' >> ~/.bashrc
```

**Kernel parameters**:
```bash
# For production deployments
sudo sysctl -w net.core.rmem_max=134217728
sudo sysctl -w net.core.wmem_max=134217728
sudo sysctl -w net.ipv4.tcp_max_syn_backlog=4096
```

---

## 🐳 Docker Setup

### Docker Compose (Multi-container)

```yaml
# docker-compose.yml
version: '3.9'

services:
  loom-server:
    image: ghuntley/loom:latest
    ports:
      - "8080:8080"
    environment:
      RUST_LOG: info
      LOOM_BUFFER_SIZE: 50000
    volumes:
      - ./data:/data
    restart: unless-stopped

  # Optional: Database service
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: loom
      POSTGRES_PASSWORD: securepassword
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

volumes:
  postgres_data:
```

**Run**:
```bash
docker-compose up -d
docker-compose logs -f loom-server
```

---

## 🌐 Web UI Installation

### Standalone Web UI

```bash
# Navigate to web directory
cd crates/loom-web

# Install dependencies
npm ci

# Build for production
npm run build

# Serve (requires server running)
npm run serve
```

### Integrated with Server

The web UI is automatically served by `loom-server` at `http://localhost:8080`

---

## 🔒 Secure Installation

### TLS/HTTPS Setup

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Run server with TLS
loom-server --port 8443 --tls-cert cert.pem --tls-key key.pem
```

### Authentication Setup

```bash
# Initialize auth database
loom auth init

# Create admin user
loom auth add-user --name admin --role admin --password <secure-password>

# Create service account
loom auth add-service-account --name api-client
```

---

## 🚀 Post-Installation

### First Steps

1. **Verify Installation**:
   ```bash
   loom --version
   loom-server --help
   ```

2. **Start Server**:
   ```bash
   loom-server --port 8080
   ```

3. **Access Web UI**:
   Open `http://localhost:8080` in browser

4. **Run First Trace**:
   ```bash
   loom trace --help
   loom trace -o trace.json
   ```

5. **View Documentation**:
   See [GETTING_STARTED.md](file:///home/ghuntley/loom/GETTING_STARTED.md)

### Configuration Files

Create `~/.config/loom/config.toml`:
```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4

[logging]
level = "info"
format = "json"

[database]
url = "postgres://localhost/loom"
pool_size = 10
```

---

## 🆘 Troubleshooting Installation

### Build Fails with "Rust not found"

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Cannot Find Binary After Installation

```bash
# Check PATH
echo $PATH

# Verify installation location
which loom

# If not found, install to system path
cargo install --path crates/loom-cli --force
```

### Permission Denied on Linux

```bash
# Fix permissions
sudo chown $USER:$USER /usr/local/bin/loom
chmod +x /usr/local/bin/loom
```

### Docker Permission Issues

```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker ps
```

### Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080

# Use different port
loom-server --port 8081
```

---

## 📚 Next Steps

After installation:

1. **Read**: [GETTING_STARTED.md](file:///home/ghuntley/loom/GETTING_STARTED.md)
2. **Explore**: [QUICK_START_QUERY_BRIDGE.md](file:///home/ghuntley/loom/QUICK_START_QUERY_BRIDGE.md)
3. **Deploy**: [DEPLOYMENT_GUIDE.md](file:///home/ghuntley/loom/DEPLOYMENT_GUIDE.md)
4. **Develop**: [DEV_ENVIRONMENT_SETUP.md](file:///home/ghuntley/loom/DEV_ENVIRONMENT_SETUP.md)

---

## 📞 Support

- **Installation Issues**: [GitHub Issues](https://github.com/ghuntley/loom/issues)
- **Troubleshooting**: [TROUBLESHOOTING_DEPLOYMENT.md](file:///home/ghuntley/loom/TROUBLESHOOTING_DEPLOYMENT.md)
- **Documentation**: [FINAL_DOCUMENTATION_INDEX.md](file:///home/ghuntley/loom/FINAL_DOCUMENTATION_INDEX.md)

---

**Installation Verified**: ✅ Complete
Last Updated: 2025-12-22
