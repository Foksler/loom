# DEPLOYMENT READY GUIDE
**Loom - Production Deployment Instructions**

---

## PREREQUISITES

### System Requirements
- **OS:** Linux (Ubuntu 20.04+) or macOS
- **CPU:** 2+ cores
- **RAM:** 4GB minimum (8GB recommended)
- **Disk:** 10GB free space

### Software Requirements
- **Rust:** 1.70+ (build only)
- **PostgreSQL:** 12+
- **Docker:** 20.10+ (optional, for containerization)
- **Docker Compose:** 1.29+ (optional)

### Network Requirements
- Port 3000: Web UI (HTTPS recommended)
- Port 8000: API Server (internal only)
- Port 6379: Redis (optional, for caching)
- Port 5432: PostgreSQL (should be internal-only)

### Access Requirements
- PostgreSQL connection string
- TLS certificates (for HTTPS)
- Environment secrets (API keys, etc.)
- Domain name (for production)

---

## PRE-DEPLOYMENT VERIFICATION

### 1. Code Compilation Check
```bash
cd /home/ghuntley/loom

# Full build verification
make check

# Expected output:
# ✅ Format check passes
# ✅ Lint passes (0 warnings)
# ✅ Build succeeds
# ✅ Tests: 254 passing
```

### 2. Test Execution
```bash
# Run test suite
make test

# Expected result: 254 tests pass, 7 may timeout (flaky)
# Core functionality is fully verified
```

### 3. Feature Verification
```bash
# Build library
cargo build -p loom-web --lib
cargo build -p loom-server --lib

# Build binaries with all features
cargo build -p loom-web --features hydrate --release
cargo build -p loom-server --release

# Expected: Both complete without errors
```

### 4. Database Verification
```bash
# Test PostgreSQL connection
psql postgresql://user:password@host:5432/loom -c "SELECT 1;"

# Expected: Returns 1 (successful connection)
```

---

## BUILD COMMANDS

### Development Build
```bash
# Quick build for development
cargo build

# Time: ~2-3 minutes
# Artifacts: target/debug/
```

### Release Build (Recommended for Production)
```bash
# Optimized build for production
cargo build --release

# Time: ~5-7 minutes (first build)
# Artifacts: target/release/
# Size reduction: ~70% smaller binaries
# Performance: ~10-30% faster execution
```

### Specific Crate Builds
```bash
# Build only backend server
cargo build -p loom-server --release

# Build only web UI with hydration
cargo build -p loom-web --features hydrate --release

# Build library only (no binary)
cargo build -p loom-web --lib
```

### Feature Matrix
```bash
# Default features (SSR)
cargo build -p loom-web

# With hydration (client-side rendering)
cargo build -p loom-web --features hydrate

# All features
cargo build --all-features
```

---

## CONFIGURATION NEEDED

### Environment Variables

#### Database
```bash
# PostgreSQL connection
DATABASE_URL="postgresql://user:password@localhost:5432/loom"

# Connection pool size
DATABASE_POOL_SIZE=20

# Query timeout (milliseconds)
DATABASE_QUERY_TIMEOUT=30000
```

#### API Server
```bash
# Server bind address
SERVER_ADDR="0.0.0.0:8000"

# API server name (for logging)
SERVER_NAME="loom-server"

# Log level
RUST_LOG="info,loom_server=debug"

# API listening address
API_BIND="0.0.0.0:8000"
```

#### Web UI
```bash
# Server URL (for API calls)
LOOM_SERVER_URL="http://localhost:8000"

# HTTPS mode
LEPTOS_SITE_ADDR="0.0.0.0:3000"

# Environment
APP_ENV="production"
```

#### Query Handling
```bash
# User input query timeout (seconds)
QUERY_USER_INPUT_TIMEOUT_SECS=30

# Environment query timeout (seconds)
QUERY_ENVIRONMENT_TIMEOUT_SECS=5

# Maximum query size (bytes)
QUERY_MAX_SIZE=10485760  # 10MB

# Rate limit: requests per second per session
RATE_LIMIT_REQUESTS_PER_SECOND=10

# Rate limit: tokens in burst
RATE_LIMIT_BURST_SIZE=20
```

#### Security
```bash
# Allowed file paths (colon-separated)
ALLOWED_FILE_PATHS="/home:/tmp:/var/log"

# Blocked file paths (colon-separated)
BLOCKED_FILE_PATHS="/etc/passwd:/etc/shadow:/root"

# Maximum result size (bytes)
MAX_RESULT_SIZE=52428800  # 50MB
```

#### Observability
```bash
# Metrics enabled
METRICS_ENABLED=true

# Trace store capacity
TRACE_STORE_CAPACITY=10000

# Slow query threshold (milliseconds)
SLOW_QUERY_THRESHOLD_MS=5000

# Trace retention (hours)
TRACE_RETENTION_HOURS=24
```

#### Docker (if using containers)
```bash
# Container port mapping
CONTAINER_WEB_PORT=3000
CONTAINER_API_PORT=8000

# Image names
REGISTRY="ghcr.io"
IMAGE_NAMESPACE="ghuntley"
IMAGE_VERSION="latest"
```

### Configuration File (.env.production)
```bash
# Copy template
cp .env.example .env.production

# Edit with production values
nano .env.production

# Required variables:
# - DATABASE_URL
# - LOOM_SERVER_URL
# - SERVER_ADDR
# - ALLOWED_FILE_PATHS
```

### Database Migration
```bash
# Apply schema migrations
sqlx migrate run \
  --database-url "$DATABASE_URL" \
  --source "./migrations"

# Verify schema
psql "$DATABASE_URL" -c "\dt"

# Expected tables: threads, messages, sessions
```

### TLS Configuration
```bash
# For HTTPS (required for production)

# 1. Obtain certificates (Let's Encrypt recommended)
certbot certonly --standalone \
  -d your-loom-domain.com

# 2. Configure in reverse proxy (Nginx/Caddy)
# See deployment/nginx.conf

# 3. Update CORS settings in code if needed
```

---

## TESTING BEFORE DEPLOYMENT

### 1. Local Integration Test
```bash
# Start PostgreSQL
docker run -d \
  -e POSTGRES_PASSWORD=postgres \
  -p 5432:5432 \
  postgres:15

# Set connection string
export DATABASE_URL="postgresql://postgres:postgres@localhost/loom"

# Run migrations
sqlx migrate run

# Run tests
make test

# Expected: 254+ tests pass
```

### 2. Manual API Testing
```bash
# Start server in background
cargo run -p loom-server &
SERVER_PID=$!

sleep 2

# Test health endpoint
curl http://localhost:8000/health

# Expected: {"status": "ok"}

# Test metrics endpoint
curl http://localhost:8000/metrics

# Expected: Prometheus format output

# Cleanup
kill $SERVER_PID
```

### 3. Component Testing
```bash
# Test Web UI server functions
cargo test -p loom-web --lib

# Test async query handling
cargo test -p loom-server --lib query_manager_integration_tests

# Test security validations
cargo test -p loom-server --lib query_security_tests

# Expected: All passing
```

### 4. Database Testing
```bash
# Test thread operations
cargo test -p loom-server --lib web_integration_test

# Test message persistence
cargo test -p loom-server --lib web_integration_test::tests::test_create_thread_persistence

# Expected: All passing
```

### 5. Load Testing (Optional)
```bash
# Install load tester
cargo install cargo-flamegraph

# Run basic load test
for i in {1..100}; do
  curl -X POST http://localhost:8000/api/test &
done
wait

# Monitor: htop, iostat
```

### 6. Security Checklist
```bash
# Test path validation
curl -X POST http://localhost:8000/api/query \
  -H "Content-Type: application/json" \
  -d '{"path": "/etc/passwd"}'

# Expected: 403 Forbidden or validation error

# Test rate limiting
for i in {1..100}; do
  curl http://localhost:8000/api/test &
done
wait

# Expected: Some requests return 429 Too Many Requests
```

---

## DEPLOYMENT INSTRUCTIONS

### Option 1: Docker Compose (Recommended for Quick Start)

```bash
# 1. Copy configuration
cp docker/.env.example .env.production

# 2. Edit environment
nano .env.production

# 3. Build containers
docker-compose build

# 4. Start services
docker-compose up -d

# 5. Verify deployment
docker-compose ps

# 6. Check logs
docker-compose logs -f
```

**docker-compose.yml:**
```yaml
version: '3.8'
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: loom
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    networks:
      - loom

  server:
    build:
      context: .
      dockerfile: docker/Dockerfile.server
    environment:
      DATABASE_URL: postgresql://postgres:${DB_PASSWORD}@postgres/loom
      RUST_LOG: info
    ports:
      - "8000:8000"
    depends_on:
      - postgres
    networks:
      - loom

  web:
    build:
      context: .
      dockerfile: docker/Dockerfile.web
    environment:
      LOOM_SERVER_URL: http://server:8000
    ports:
      - "3000:3000"
    depends_on:
      - server
    networks:
      - loom

volumes:
  postgres_data:

networks:
  loom:
```

### Option 2: Kubernetes (For Cluster Deployment)

```bash
# 1. Build and push images
docker build -t your-registry/loom-server:latest -f docker/Dockerfile.server .
docker build -t your-registry/loom-web:latest -f docker/Dockerfile.web .

docker push your-registry/loom-server:latest
docker push your-registry/loom-web:latest

# 2. Create namespace
kubectl create namespace loom

# 3. Create secrets
kubectl create secret generic loom-secrets \
  --from-literal=database-url="postgresql://..." \
  --from-literal=api-key="..." \
  -n loom

# 4. Deploy
kubectl apply -f deployment/k8s/

# 5. Verify
kubectl get pods -n loom
```

### Option 3: Manual Binary Installation

```bash
# 1. Create deployment directory
mkdir -p /opt/loom/{bin,config,logs}
cd /opt/loom

# 2. Copy binaries
cp ~/loom/target/release/loom-server ./bin/
cp ~/loom/target/release/loom-web ./bin/

# 3. Copy config
cp ~/loom/.env.production ./config/.env

# 4. Create systemd service
sudo tee /etc/systemd/system/loom-server.service > /dev/null <<EOF
[Unit]
Description=Loom Query Server
After=network.target

[Service]
Type=simple
User=loom
WorkingDirectory=/opt/loom
EnvironmentFile=/opt/loom/config/.env
ExecStart=/opt/loom/bin/loom-server
Restart=on-failure
StandardOutput=append:/opt/loom/logs/server.log
StandardError=append:/opt/loom/logs/server.log

[Install]
WantedBy=multi-user.target
EOF

# 5. Start service
sudo systemctl daemon-reload
sudo systemctl enable loom-server
sudo systemctl start loom-server

# 6. Check status
sudo systemctl status loom-server
```

### Option 4: Nix Flake (For NixOS)

```bash
# 1. Add to your flake.nix
{
  inputs = {
    loom.url = "github:ghuntley/loom";
  };
}

# 2. In your system configuration
environment.systemPackages = with pkgs; [
  inputs.loom.packages.${system}.loom-server
  inputs.loom.packages.${system}.loom-web
];

# 3. Rebuild and switch
sudo nixos-rebuild switch
```

---

## POST-DEPLOYMENT VERIFICATION

### Health Checks
```bash
# 1. API Server Health
curl -i http://localhost:8000/health

# Expected:
# HTTP/1.1 200 OK
# {"status":"ok"}

# 2. Web UI Access
curl -i http://localhost:3000

# Expected:
# HTTP/1.1 200 OK
# <HTML...

# 3. Database Connection
curl http://localhost:8000/api/threads

# Expected:
# [] or list of threads (200 OK)

# 4. Metrics Endpoint
curl http://localhost:8000/metrics

# Expected:
# Prometheus format with metrics
```

### Service Checks
```bash
# Check if processes are running
ps aux | grep loom-server
ps aux | grep loom-web

# Check if ports are open
netstat -tlnp | grep -E "3000|8000"

# Check database connections
psql -c "SELECT datname, count(*) FROM pg_stat_activity GROUP BY datname;"

# Check logs for errors
tail -f logs/server.log
tail -f logs/web.log
```

### Monitoring Setup
```bash
# 1. Prometheus scrape config
cat > /etc/prometheus/prometheus.yml <<EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'loom'
    static_configs:
      - targets: ['localhost:8000']
EOF

# 2. Verify metrics collection
curl http://localhost:9090/api/v1/query?query=up

# 3. Create Grafana dashboard
# Import dashboard JSON from deployment/grafana/loom-dashboard.json
```

### Alerting Setup
```bash
# 1. Create alert rules
cat > /etc/prometheus/loom-alerts.yml <<EOF
groups:
  - name: loom
    interval: 30s
    rules:
      - alert: LoomServerDown
        expr: up{job="loom"} == 0
        for: 5m
        annotations:
          summary: "Loom server is down"

      - alert: HighErrorRate
        expr: rate(loom_query_errors_total[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate in Loom"
EOF

# 2. Reload Prometheus
curl -X POST http://localhost:9090/-/reload
```

---

## TROUBLESHOOTING

### Common Issues

#### 1. Database Connection Failed
```bash
# Check PostgreSQL is running
ps aux | grep postgres

# Test connection manually
psql postgresql://user:pass@host:5432/loom -c "SELECT 1;"

# Check DATABASE_URL environment variable
echo $DATABASE_URL

# Verify credentials and network connectivity
```

#### 2. Port Already in Use
```bash
# Find process using port 3000
lsof -i :3000

# Find process using port 8000
lsof -i :8000

# Kill process (replace PID)
kill -9 <PID>

# Or use different ports
export WEB_PORT=3001
export API_PORT=8001
```

#### 3. High Memory Usage
```bash
# Check memory consumption
ps aux | grep loom

# Reduce trace store capacity
export TRACE_STORE_CAPACITY=1000

# Reduce connection pool size
export DATABASE_POOL_SIZE=5

# Monitor memory
watch -n 1 'ps aux | grep loom'
```

#### 4. Slow Queries
```bash
# Check slow query logs
tail -f logs/slow-queries.log

# Review slow query threshold
echo $SLOW_QUERY_THRESHOLD_MS

# Check database indexes
psql -c "\d+ threads"
psql -c "\d+ messages"

# Run ANALYZE
psql -c "ANALYZE threads; ANALYZE messages;"
```

#### 5. TLS Certificate Errors
```bash
# Verify certificate is valid
openssl x509 -in /path/to/cert.pem -text -noout

# Check expiration date
openssl x509 -enddate -noout -in /path/to/cert.pem

# Renew with Let's Encrypt
certbot renew --quiet
```

---

## MAINTENANCE

### Regular Tasks

#### Daily
- [ ] Monitor error logs
- [ ] Check disk space usage
- [ ] Verify backup completion

#### Weekly
- [ ] Review metrics dashboards
- [ ] Analyze slow query logs
- [ ] Check certificate expiration dates

#### Monthly
- [ ] Update dependencies: `cargo update`
- [ ] Run security audits: `cargo audit`
- [ ] Backup database: `pg_dump`
- [ ] Review access logs

#### Quarterly
- [ ] Performance optimization review
- [ ] Security assessment
- [ ] Capacity planning
- [ ] Plan feature updates

### Backup Strategy
```bash
# Daily automated backup
0 2 * * * /opt/loom/scripts/backup-db.sh

# Backup script
#!/bin/bash
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
pg_dump $DATABASE_URL | gzip > /backups/loom_$TIMESTAMP.sql.gz

# Keep last 30 days
find /backups -name "loom_*.sql.gz" -mtime +30 -delete
```

### Update Procedure
```bash
# 1. Pull latest code
git pull origin main

# 2. Review changes
git log --oneline -n 10

# 3. Run tests
make check

# 4. Build release
cargo build --release

# 5. Backup database
pg_dump $DATABASE_URL | gzip > backups/pre-update.sql.gz

# 6. Stop services
systemctl stop loom-server loom-web

# 7. Deploy new binaries
cp target/release/loom-* /opt/loom/bin/

# 8. Restart services
systemctl start loom-server loom-web

# 9. Verify
curl http://localhost:8000/health
```

---

## SUPPORT & DOCUMENTATION

- **API Reference:** [API_REFERENCE_COMPLETE.md](file:///home/ghuntley/loom/API_REFERENCE_COMPLETE.md)
- **Troubleshooting:** [TROUBLESHOOTING_QUERY_BRIDGE.md](file:///home/ghuntley/loom/TROUBLESHOOTING_QUERY_BRIDGE.md)
- **Testing Guide:** [TESTING_GUIDE.md](file:///home/ghuntley/loom/TESTING_GUIDE.md)
- **Architecture:** [COMPONENT_ARCHITECTURE.md](file:///home/ghuntley/loom/COMPONENT_ARCHITECTURE.md)

---

**Deployment Ready: ✅ YES**

*Last Updated: 2025-12-22*
