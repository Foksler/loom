# Deployment Automation & Production Configuration

Complete deployment automation and production-ready configuration for Loom.

## Quick Start

```bash
# 1. Build release binary
./deploy/build.sh release

# 2. Deploy to staging
./deploy/deploy.sh --env staging --host staging.example.com --user deploy

# 3. Verify health
./deploy/health-check.sh --url https://staging.example.com

# 4. Rollback if needed
./deploy/rollback.sh --host staging.example.com --user deploy --verify
```

## Deployment Scripts

### 1. **build.sh** - Release Build
Builds optimized Rust binaries with comprehensive checks.

**Features:**
- Pre-build validation (Rust toolchain, workspace integrity)
- Clippy linting and format checks
- Full test suite execution
- Build metadata generation
- Artifact listing

**Usage:**
```bash
./deploy/build.sh release
./deploy/build.sh debug
```

**Output:**
- Release binary in `target/release/`
- Build metadata: `build-info.json`

### 2. **deploy.sh** - Environment Deployment
Deploys to target environment via SSH with multiple strategies.

**Deployment Methods:**
- Docker (container-based)
- Binary (direct executable)
- Systemd (service-based)

**Usage:**
```bash
# Docker deployment
./deploy/deploy.sh \
  --env production \
  --host prod.example.com \
  --user deploy \
  --method docker \
  --version latest

# Binary deployment
./deploy/deploy.sh \
  --env staging \
  --host staging.example.com \
  --user deploy \
  --method binary \
  --no-backup

# Dry-run
./deploy/deploy.sh \
  --host example.com \
  --user deploy \
  --dry-run
```

**Features:**
- Automated backup creation
- SSH-based secure deployment
- Pre-deployment connectivity checks
- Deployment directory validation

### 3. **health-check.sh** - Deployment Verification
Verifies service health and operational status.

**Health Checks:**
- HTTP health endpoint (`/health`)
- Metrics collection (`/metrics`)
- System resources (CPU, memory, disk)
- Response time metrics

**Usage:**
```bash
./deploy/health-check.sh

./deploy/health-check.sh \
  --url http://prod.example.com:8080 \
  --timeout 30 \
  --retries 5 \
  --interval 5 \
  --verbose
```

**Metrics Collected:**
- Response time (min, max, average)
- HTTP status codes
- Service availability
- System resource usage

### 4. **rollback.sh** - Deployment Rollback
Rolls back to previous deployment on failure.

**Features:**
- Automatic backup detection
- Multiple rollback methods
- Health verification post-rollback
- Confirmation prompts for safety

**Usage:**
```bash
# Rollback to latest backup
./deploy/rollback.sh \
  --host prod.example.com \
  --user deploy \
  --verify

# Rollback to specific backup
./deploy/rollback.sh \
  --host prod.example.com \
  --user deploy \
  --backup-name loom-backup-20240101-120000 \
  --method docker

# Dry-run rollback
./deploy/rollback.sh \
  --host prod.example.com \
  --user deploy \
  --dry-run
```

## Configuration Templates

### 1. **.env.example** - Environment Variables
Comprehensive template for all configuration options.

**Sections:**
- Application settings
- Server configuration
- Database connectivity
- Cache settings
- API keys and secrets
- Monitoring and observability
- Security parameters
- Storage configuration
- Email settings
- Backup configuration
- Performance tuning
- Feature flags
- External services
- Deployment settings

**Usage:**
```bash
cp deploy/.env.example /etc/loom/.env
# Edit /etc/loom/.env with your values
chmod 600 /etc/loom/.env
```

### 2. **docker-compose.yml** - Multi-Service Setup
Production-ready Docker Compose orchestration.

**Services:**
- PostgreSQL 16 (database)
- Redis 7 (cache)
- Loom Server (main application)
- Prometheus (metrics)
- Grafana (dashboards)
- Jaeger (distributed tracing)
- Nginx (reverse proxy)

**Features:**
- Health checks for all services
- Volume persistence
- Network isolation
- Security opt-in (no-new-privileges)
- Capability dropping

**Usage:**
```bash
# Start all services
docker-compose -f deploy/docker-compose.yml up -d

# View logs
docker-compose -f deploy/docker-compose.yml logs -f loom-server

# Stop all services
docker-compose -f deploy/docker-compose.yml down

# Scale services
docker-compose -f deploy/docker-compose.yml up -d --scale loom-server=3
```

### 3. **kubernetes.yaml** - K8s Deployment
Production Kubernetes manifests with best practices.

**Components:**
- Namespace isolation
- ConfigMaps (configuration)
- Secrets (sensitive data)
- StatefulSets (PostgreSQL, Redis)
- Deployments (Loom server)
- Services (networking)
- Ingress (external access)
- HorizontalPodAutoscaler (scaling)
- PodDisruptionBudget (availability)
- ServiceMonitor (Prometheus integration)

**Features:**
- High availability setup (3 replicas)
- Pod anti-affinity for distribution
- Resource quotas and limits
- Security contexts (non-root user)
- ReadOnly root filesystem
- Network policies ready

**Usage:**
```bash
# Deploy to Kubernetes
kubectl apply -f deploy/kubernetes.yaml

# Check deployment status
kubectl -n loom get deployments
kubectl -n loom get pods

# View logs
kubectl -n loom logs -f deployment/loom-server

# Scale deployment
kubectl -n loom scale deployment loom-server --replicas=5

# Rollback if needed
kubectl -n loom rollout undo deployment/loom-server
```

### 4. **nginx.conf** - Reverse Proxy
Production-grade Nginx configuration.

**Features:**
- SSL/TLS with modern ciphers
- Security headers (HSTS, CSP, X-Frame-Options)
- CORS handling
- Rate limiting
- Gzip compression
- Request buffering
- WebSocket support
- Error handling
- Monitoring endpoint

**Sections:**
- HTTP to HTTPS redirect
- Health check endpoint (unrestricted)
- Metrics endpoint (IP-restricted)
- API endpoints (rate-limited)
- WebSocket endpoints (persistent)
- Static file caching
- Error pages

**Usage:**
```bash
# Test configuration
nginx -t -c deploy/nginx.conf

# Reload Nginx
nginx -s reload

# Monitor status
curl http://127.0.0.1:8888/nginx_status
```

**Configuration Details:**
- Worker processes: auto
- Worker connections: 4096
- Keepalive timeout: 65s
- Client max body: 20MB
- Upstream health checks: 3 max_fails

### 5. **loom.service** - Systemd Unit
Secure systemd service configuration.

**Features:**
- Automatic restart on failure
- Security hardening:
  - PrivateTmp=yes
  - NoNewPrivileges=yes
  - ProtectSystem=strict
  - ProtectHome=yes
  - MemoryDenyWriteExecute=yes
- Resource limits:
  - Memory limit: 1GB
  - CPU quota: 50%
  - Max open files: 65535
- Process management
- Journal logging

**Installation:**
```bash
sudo cp deploy/loom.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable loom
sudo systemctl start loom

# Monitor
sudo systemctl status loom
sudo journalctl -u loom -f
```

## Monitoring & Logging

### 1. **logging.yaml** - Structured Logging
Comprehensive logging configuration with multiple outputs.

**Log Levels:**
- Default: info
- Module overrides for fine-grained control
- Sampling configuration (reduce volume)

**Outputs:**
- Console (development)
- File rotation (production)
- Error file (separate error tracking)
- Audit log (security events)

**Features:**
- JSON output format
- Trace/span correlation IDs
- Sensitive data redaction
- GDPR compliance support
- OpenTelemetry integration
- Alert thresholds
- Performance configuration

**Usage:**
```yaml
# Configure in application
log_level: info
format: json
outputs:
  - type: file
    path: /var/log/loom/loom.log
    rotation:
      max_size: 100M
      max_backups: 10
```

### 2. **prometheus.yml** - Metrics Collection
Prometheus configuration for all services.

**Scrape Targets:**
- Prometheus self-monitoring
- Loom server metrics
- PostgreSQL metrics
- Redis metrics
- Node metrics (hardware)
- Nginx metrics
- Docker/cAdvisor metrics

**Alert Rules:**
- Server down detection
- High error rates (>5%)
- High response times (>1s P95)
- High CPU/memory usage
- Database pool exhaustion
- Redis unavailability
- Disk space warnings
- Request rate spikes

**Usage:**
```bash
# Validate configuration
promtool check config prometheus.yml

# View metrics
curl http://localhost:9090/api/v1/query?query=up

# Query metrics
curl "http://localhost:9090/api/v1/query_range?query=rate(http_requests_total[5m])&start=...&end=...&step=15s"
```

### 3. **grafana-dashboard.json** - Visualization
Pre-built Grafana dashboard for monitoring.

**Dashboard Panels:**
- Server status (stat card)
- Uptime (duration)
- Request rate (graph)
- HTTP status distribution
- Error rate tracking
- Response time (P95)
- CPU/memory usage
- Database connections
- Database query performance
- Cache hit ratio
- Active connections
- Logs integration

**Usage:**
```bash
# Import dashboard
# In Grafana: Dashboard → Import → Upload JSON file

# Or configure as provisioned dashboard
cp deploy/grafana-dashboard.json /etc/grafana/provisioning/dashboards/
```

## Deployment Workflows

### Standard Production Deployment

```bash
#!/bin/bash
set -e

# 1. Build
echo "Building release binary..."
./deploy/build.sh release

# 2. Test
echo "Running tests..."
cargo test --release

# 3. Deploy to staging
echo "Deploying to staging..."
./deploy/deploy.sh \
  --env staging \
  --host staging.example.com \
  --user deploy \
  --method docker

# 4. Health check staging
echo "Checking staging health..."
./deploy/health-check.sh --url https://staging.example.com

# 5. Deploy to production (after approval)
echo "Deploying to production..."
./deploy/deploy.sh \
  --env production \
  --host prod.example.com \
  --user deploy \
  --method docker \
  --version $(cargo pkgid | cut -d# -f2)

# 6. Verify production
echo "Verifying production..."
./deploy/health-check.sh --url https://prod.example.com

echo "✓ Deployment complete!"
```

### Blue-Green Deployment

```bash
# Deploy new version to green environment
./deploy/deploy.sh --host green.example.com --user deploy

# Verify green environment
./deploy/health-check.sh --url https://green.example.com

# Switch traffic (update load balancer/DNS)
# ...

# Keep blue as rollback target
# Deploy new version when ready
./deploy/deploy.sh --host blue.example.com --user deploy
```

### Canary Deployment

```bash
# Deploy to canary instances (10% traffic)
./deploy/deploy.sh --host canary1.example.com --user deploy
./deploy/deploy.sh --host canary2.example.com --user deploy

# Monitor metrics for 24 hours
# If successful, deploy to remaining instances
./deploy/deploy.sh --host prod1.example.com --user deploy
./deploy/deploy.sh --host prod2.example.com --user deploy
```

## Troubleshooting

### Health Check Failures

```bash
# Verbose health check
./deploy/health-check.sh --verbose

# Check service logs
docker logs loom-server
# or
journalctl -u loom -n 50

# Test endpoint directly
curl -v http://localhost:8080/health

# Check port binding
netstat -tlnp | grep 8080
```

### Deployment Issues

```bash
# Test SSH connectivity
ssh -v deploy@host "true"

# Check deployment directory
ssh deploy@host "ls -la /opt/loom"

# View deployment status
ssh deploy@host "systemctl status loom"

# Check recent logs
ssh deploy@host "journalctl -u loom -n 100"

# Rollback
./deploy/rollback.sh --host example.com --user deploy --verify
```

### Database Issues

```bash
# Check PostgreSQL
docker exec loom-postgres psql -U loom -d loom -c "\dt"

# View slow queries
docker exec loom-postgres psql -U loom -d loom -c \
  "SELECT query, calls, total_time FROM pg_stat_statements ORDER BY total_time DESC;"

# Reset statistics
docker exec loom-postgres psql -U loom -d loom -c "SELECT pg_stat_statements_reset();"
```

### Performance Issues

```bash
# Check metrics
curl http://localhost:9090/metrics | grep -E "http_request|db_query"

# Identify slow endpoints
curl "http://localhost:9090/api/v1/query?query=\
  histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) by (path)"

# View top resource consumers
docker stats --no-stream
```

## Security Considerations

1. **Secrets Management:**
   - Use environment variables for sensitive data
   - Store `.env` files outside version control
   - Rotate API keys regularly
   - Use secret management tools (Vault, Sealed Secrets)

2. **TLS/SSL:**
   - Use Let's Encrypt for certificates
   - Enable HSTS headers
   - Configure strong ciphers
   - Regular certificate renewal

3. **Access Control:**
   - Restrict SSH access
   - Use SSH keys (no passwords)
   - Implement IP whitelisting
   - Regular access audits

4. **Data Protection:**
   - Enable encryption at rest
   - Use TLS for data in transit
   - Regular backups with testing
   - GDPR-compliant data handling

5. **Monitoring:**
   - Enable audit logging
   - Monitor for suspicious activity
   - Alert on anomalies
   - Regular security reviews

## Backup & Recovery

```bash
# Backup database
docker exec loom-postgres pg_dump -U loom loom > backup.sql

# Backup configuration
tar czf loom-config-backup.tar.gz /etc/loom

# Restore database
docker exec -i loom-postgres psql -U loom loom < backup.sql

# Restore configuration
tar xzf loom-config-backup.tar.gz
```

## References

- [Loom Repository](https://github.com/ghuntley/loom)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Docker Documentation](https://docs.docker.com/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Nginx Documentation](https://nginx.org/en/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
