# Loom Web Production Deployment Package

**Version**: 0.1.0 (commit: 153aba0)  
**Build Date**: 2025-12-22  
**Status**: Ready for deployment preparation

## Overview

Complete deployment package for Loom Web, a Leptos-based isomorphic SPA for AI-powered code search and analysis.

---

## 1. System Requirements

### Server Requirements
- **OS**: Linux (Ubuntu 20.04+), macOS 12+, or compatible Unix systems
- **RAM**: Minimum 2GB, recommended 4GB+
- **CPU**: 2+ cores recommended
- **Disk**: 500MB minimum (including WASM bundle)

### Deployment Options

#### Docker (Recommended)
- **Docker**: 20.10+
- **Docker Compose**: 1.29+ (for orchestration)
- **Container Registry**: Docker Hub or private registry

#### Bare Metal
- **Rust Runtime**: Not required (compiled binary only)
- **Node.js**: Not required (pre-built assets)
- **Reverse Proxy**: Nginx/Caddy recommended

### Browser Support
- Chrome/Edge 88+
- Firefox 78+
- Safari 14+
- WASM support required

---

## 2. Environment Variables

### Required Variables
```bash
# Server Configuration
LOOM_WEB_PORT=3000                    # Server port (default: 3000)
LOOM_WEB_HOST=0.0.0.0                # Bind address (default: 127.0.0.1)
LOOM_ENV=production                  # Environment name

# API Configuration
LOOM_API_BASE_URL=http://localhost:8000  # Backend API endpoint
LOOM_API_TIMEOUT=30                  # API request timeout (seconds)

# Security
LOOM_SESSION_SECRET=$(openssl rand -base64 32)  # Session encryption key (generate)
LOOM_CORS_ORIGIN=https://yourdomain.com        # CORS allowed origins

# Logging
RUST_LOG=info,loom_web=debug         # Logging level
LOG_FORMAT=json                       # json or text
```

### Optional Variables
```bash
# Performance
LOOM_WEB_WORKERS=4                   # Worker threads (auto-detected if not set)
LOOM_CACHE_EXPIRY=3600               # Cache TTL in seconds

# Features
LOOM_ENABLE_METRICS=true             # Enable Prometheus metrics
LOOM_ENABLE_TRACING=true             # Enable distributed tracing
LOOM_METRICS_PORT=9090               # Metrics endpoint port

# Development (do NOT use in production)
LOOM_DEBUG=false                      # Debug mode
LOOM_HOT_RELOAD=false                # Hot reload (dev only)
```

---

## 3. Configuration Options

### Application Configuration

#### Server Configuration (`server.toml`)
```toml
[server]
port = 3000
host = "0.0.0.0"
workers = 4
timeout_sec = 30

[logging]
level = "info"
format = "json"
output = "stdout"

[security]
enable_csrf = true
secure_cookies = true
same_site = "Lax"

[features]
enable_metrics = true
enable_tracing = true
```

#### API Configuration
```bash
# Backend integration
LOOM_API_BASE_URL=https://api.loom.local
LOOM_API_RETRY_COUNT=3
LOOM_API_RETRY_DELAY=1000      # milliseconds

# WebSocket for streaming
LOOM_WEBSOCKET_URL=wss://api.loom.local/ws
LOOM_WEBSOCKET_PING_INTERVAL=30000
```

#### Feature Flags
```bash
# UI Features
LOOM_ENABLE_THREAD_SEARCH=true
LOOM_ENABLE_QUERY_HISTORY=true
LOOM_ENABLE_CODE_SHARING=false
LOOM_ENABLE_ANALYTICS=true

# Advanced
LOOM_ENABLE_CUSTOM_TOOLS=false
LOOM_ENABLE_PLUGIN_SYSTEM=false
```

---

## 4. Scaling Considerations

### Horizontal Scaling

#### Load Balancing Strategy
```nginx
upstream loom_web {
    least_conn;  # Load balancing algorithm
    server loom-web-1:3000 weight=1;
    server loom-web-2:3000 weight=1;
    server loom-web-3:3000 weight=1;
}

server {
    listen 443 ssl http2;
    server_name loom.example.com;

    location / {
        proxy_pass http://loom_web;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

#### Docker Swarm Scaling
```bash
docker service create \
  --name loom-web \
  --replicas 3 \
  --port 3000:3000 \
  -e LOOM_WEB_WORKERS=4 \
  -e LOOM_API_BASE_URL=http://loom-api:8000 \
  ghuntley/loom-web:latest
```

#### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loom-web
spec:
  replicas: 3
  selector:
    matchLabels:
      app: loom-web
  template:
    metadata:
      labels:
        app: loom-web
    spec:
      containers:
      - name: loom-web
        image: ghuntley/loom-web:latest
        ports:
        - containerPort: 3000
        env:
        - name: LOOM_WEB_WORKERS
          value: "4"
        - name: LOOM_API_BASE_URL
          value: "http://loom-api:8000"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
```

### Performance Tuning

#### Connection Pooling
```bash
# API client connection pool
LOOM_API_POOL_SIZE=32
LOOM_API_POOL_TIMEOUT=10

# Database (if applicable)
DATABASE_POOL_SIZE=25
DATABASE_POOL_MIN=5
```

#### Caching Strategy
```bash
# Static asset caching
LOOM_CACHE_CONTROL="public, max-age=31536000"  # 1 year for versioned assets

# API response caching
LOOM_API_CACHE_TTL=300  # 5 minutes

# Session caching
LOOM_SESSION_CACHE_TTL=3600  # 1 hour
```

### Expected Performance Metrics

| Metric | Target |
|--------|--------|
| P95 Latency | < 100ms |
| P99 Latency | < 500ms |
| Throughput | 1000 req/s per instance |
| Error Rate | < 0.1% |
| Availability | > 99.9% |

---

## 5. Health Check Endpoints

### HTTP Health Checks

#### `/health` - Service Health
```bash
curl -s http://localhost:3000/health | jq .
```

Response:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "timestamp": "2025-12-22T10:30:45Z"
}
```

#### `/ready` - Readiness Check
```bash
curl -s http://localhost:3000/ready | jq .
```

Response:
```json
{
  "ready": true,
  "dependencies": {
    "api": "ok",
    "cache": "ok",
    "database": "ok"
  }
}
```

#### `/metrics` - Prometheus Metrics (if enabled)
```bash
curl -s http://localhost:9090/metrics | head -20
```

### Health Check Configuration

#### Docker Health Check
```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1
```

#### Kubernetes Health Probes
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 30
  timeoutSeconds: 3
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /ready
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 10
  timeoutSeconds: 2
  failureThreshold: 3
```

---

## 6. Monitoring & Logging Setup

### Structured Logging

#### Log Levels
```bash
RUST_LOG=loom_web=debug,leptos=info,axum=info
```

#### Log Format (JSON)
```json
{
  "timestamp": "2025-12-22T10:30:45.123Z",
  "level": "INFO",
  "target": "loom_web::handlers",
  "message": "Request processed",
  "request_id": "req_abc123",
  "duration_ms": 45,
  "status": 200,
  "method": "GET",
  "path": "/api/threads"
}
```

### Metrics Collection

#### Prometheus Integration
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'loom-web'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
    scrape_timeout: 10s
```

#### Key Metrics
- `http_requests_total` - Total HTTP requests
- `http_request_duration_seconds` - Request latency
- `http_requests_in_progress` - Active connections
- `api_calls_total` - Backend API calls
- `cache_hits_total` - Cache hit count
- `errors_total` - Application errors

### Log Aggregation

#### ELK Stack Configuration
```yaml
version: '3.8'
services:
  elasticsearch:
    image: docker.elastic.co/elasticsearch/elasticsearch:8.0.0
    environment:
      - discovery.type=single-node
    ports:
      - "9200:9200"
  
  logstash:
    image: docker.elastic.co/logstash/logstash:8.0.0
    volumes:
      - ./logstash.conf:/usr/share/logstash/pipeline/logstash.conf
    environment:
      - xpack.monitoring.enabled=false
  
  kibana:
    image: docker.elastic.co/kibana/kibana:8.0.0
    ports:
      - "5601:5601"
```

#### Logstash Configuration
```conf
input {
  stdin {}
}

filter {
  json { source => "message" }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "loom-web-%{+YYYY.MM.dd}"
  }
}
```

### Alerting Rules

#### Critical Alerts
```yaml
groups:
  - name: loom_web
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 1m
        annotations:
          summary: "High error rate detected"

      - alert: HighLatency
        expr: histogram_quantile(0.95, http_request_duration_seconds) > 1
        for: 5m
        annotations:
          summary: "API latency exceeds 1 second"

      - alert: ServiceDown
        expr: up{job="loom-web"} == 0
        for: 1m
        annotations:
          summary: "Loom Web service is down"
```

---

## 7. Security Considerations

### TLS/SSL Configuration
```nginx
ssl_certificate /etc/ssl/certs/loom.crt;
ssl_certificate_key /etc/ssl/private/loom.key;
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers HIGH:!aNULL:!MD5;
ssl_prefer_server_ciphers on;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
```

### CORS Policy
```bash
LOOM_CORS_ORIGIN="https://loom.example.com,https://app.loom.example.com"
LOOM_CORS_METHODS="GET,POST,PUT,DELETE"
LOOM_CORS_CREDENTIALS=true
```

### CSP Headers
```
Content-Security-Policy: 
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: https:;
  font-src 'self' data:;
  connect-src 'self' https://api.loom.example.com wss://api.loom.example.com;
```

---

## 8. Backup & Disaster Recovery

### Configuration Backup
```bash
# Backup application config
tar -czf loom-web-config-$(date +%s).tar.gz \
  /etc/loom-web/ \
  /var/lib/loom-web/

# Store in remote backup
aws s3 cp loom-web-config-*.tar.gz s3://backups/loom-web/
```

### Rollback Procedure
```bash
# Keep previous images
docker tag ghuntley/loom-web:latest ghuntley/loom-web:prev

# Rollback if needed
docker service update --image ghuntley/loom-web:prev loom-web
```

---

## File Locations

| Component | Location |
|-----------|----------|
| Binary | `target/release/loom-server` |
| Static Assets | `target/site/` |
| WASM Bundle | `target/site/pkg/` |
| CSS Styles | `target/site/css/` |
| Docker Image | `ghuntley/loom-web:latest` |
| Config | `/etc/loom-web/` |
| Logs | `/var/log/loom-web/` |

---

## Quick Start

### Docker Deployment
```bash
docker run -d \
  --name loom-web \
  -p 3000:3000 \
  -e LOOM_API_BASE_URL=http://loom-api:8000 \
  -e RUST_LOG=info \
  ghuntley/loom-web:latest
```

### Verify Deployment
```bash
curl http://localhost:3000/health
curl http://localhost:3000/ready
```

---

## Support & Troubleshooting

See [DEPLOYMENT_CHECKLIST.md](./DEPLOYMENT_CHECKLIST.md) for pre/post-deployment verification.

See [RELEASE_NOTES.md](./RELEASE_NOTES.md) for version-specific information.
