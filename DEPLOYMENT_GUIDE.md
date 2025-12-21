# Loom Web Deployment Guide

**Status**: Ready for deployment (after migration to Leptos 0.7)  
**Version**: 1.0  
**Last Updated**: December 22, 2025

## Quick Start

After completing the Leptos 0.7 migration:

```bash
# Development
cargo run --release

# Production Build (SSR-enabled)
cargo build --release -p loom-web --features ssr

# Test Build
cargo build -p loom-web

# Format & Lint
cargo fmt -p loom-web
cargo clippy -p loom-web -- -D warnings
```

## Environment Setup

### Prerequisites

- **Rust 1.70+** (via rustup)
- **Node.js 18+** (for build tooling if needed)
- **WebAssembly target**: `rustup target add wasm32-unknown-unknown`
- **Leptos CLI** (optional): `cargo install leptos_cli`

### Development Environment

1. **Install dependencies**
   ```bash
   # Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update
   rustup target add wasm32-unknown-unknown
   ```

2. **Clone and setup**
   ```bash
   git clone https://github.com/ghuntley/loom.git
   cd loom
   cargo build -p loom-web
   ```

3. **Run development server**
   ```bash
   cargo run --release
   # App available at http://localhost:3000
   ```

## Build Configurations

### 1. Development Build (Fastest)

```bash
cargo build -p loom-web
```

**Features**:
- Default: hydration enabled
- Unoptimized for speed
- Suitable for testing and development
- Bundle size: ~2-3 MB (WASM)

### 2. Release Build (Default SSR)

```bash
cargo build --release -p loom-web
```

**Features**:
- Optimized for performance
- Binary size: ~50-100 MB
- Suitable for production
- Supports client-side hydration

### 3. SSR Build (Server-Side Rendering)

```bash
cargo build --release -p loom-web --features ssr
```

**Features**:
- Server-side rendering enabled
- Axum web server included
- Faster initial page load
- Better SEO
- Binary size: ~100-150 MB

### 4. Minimal Build (WASM Only, No SSR)

```bash
cargo build --release -p loom-web --no-default-features
```

**Features**:
- Pure client-side rendering
- Smallest bundle: ~1-2 MB (WASM)
- Good for static hosting (Vercel, Netlify, etc.)
- Requires JavaScript runtime

## Deployment Strategies

### Strategy 1: Docker (Recommended for Production)

#### Dockerfile

```dockerfile
# Build stage
FROM rust:latest as builder

WORKDIR /app

# Install wasm target
RUN rustup target add wasm32-unknown-unknown

# Copy source
COPY . .

# Build with SSR
RUN cargo build --release -p loom-web --features ssr

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/loom_web .

# Expose port
EXPOSE 3000

# Set environment
ENV RUST_LOG=info

# Run server
CMD ["./loom_web"]
```

#### Build & Run

```bash
# Build image
docker build -t loom-web:latest .

# Run container
docker run -p 3000:3000 \
  -e LOOM_API_URL=https://api.example.com \
  -e RUST_LOG=info \
  loom-web:latest

# Deploy to registry
docker tag loom-web:latest your-registry/loom-web:latest
docker push your-registry/loom-web:latest
```

### Strategy 2: Static Hosting (Vercel, Netlify)

For client-only deployment:

```bash
# Build static files
cargo build --release -p loom-web --no-default-features

# Output location: target/release/loom_web.wasm
# Copy to static hosting provider

# vercel.json
{
  "buildCommand": "cargo build --release -p loom-web --no-default-features",
  "outputDirectory": "target/release",
  "env": {
    "LOOM_API_URL": "@loom_api_url"
  }
}
```

### Strategy 3: Kubernetes

#### K8s Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loom-web
  namespace: loom
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
        image: your-registry/loom-web:latest
        ports:
        - containerPort: 3000
        env:
        - name: LOOM_API_URL
          valueFrom:
            configMapKeyRef:
              name: loom-config
              key: api-url
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: loom-web-svc
  namespace: loom
spec:
  selector:
    app: loom-web
  type: LoadBalancer
  ports:
  - protocol: TCP
    port: 80
    targetPort: 3000
```

Deploy:
```bash
kubectl apply -f k8s-deployment.yaml
kubectl port-forward svc/loom-web-svc 3000:80
```

## Configuration

### Environment Variables

Create `.env` or set in deployment:

```env
# Required
LOOM_API_URL=https://api.loom.example.com

# Optional
RUST_LOG=info,loom_web=debug
LOOM_TIMEOUT_MS=30000
LOOM_ENABLE_STREAMING=true
LOOM_MAX_MESSAGE_LENGTH=10000
LOOM_FEATURES=chat,query-bridge,results
```

### Configuration File

Create `loom-web.toml` (optional):

```toml
[server]
port = 3000
host = "0.0.0.0"
workers = 4

[api]
url = "https://api.loom.example.com"
timeout_ms = 30000
retry_attempts = 3

[ui]
theme = "light"
enable_streaming = true
max_message_length = 10000

[logging]
level = "info"
json_format = true
```

## Performance Optimization

### 1. Asset Optimization

```bash
# Compress WASM binary
wasm-opt -Oz target/release/loom_web_bg.wasm -o loom_web_opt.wasm

# Brotli compression (HTTP)
brotli -9 -k loom_web_opt.wasm
```

### 2. Tailwind CSS Optimization

The build already includes Tailwind CSS with PurgeCSS enabled:

```javascript
// tailwind.config.cjs
module.exports = {
  content: [
    './src/**/*.rs',
    './index.html',
  ],
  theme: { /* ... */ },
  plugins: [],
}
```

This removes unused CSS from the final bundle.

### 3. Caching Strategy

```nginx
# nginx.conf for HTTP caching
location /pkg/ {
    expires 1y;
    add_header Cache-Control "public, immutable";
}

location / {
    expires 1h;
    add_header Cache-Control "public, must-revalidate";
}
```

### 4. CDN Setup

For geographically distributed users:

```bash
# Serve from CloudFlare, AWS CloudFront, or Fastly
# Configure origin: your-app-domain.com
# Cache assets in /pkg/ for 365 days
# Cache HTML for 1 hour
```

## Monitoring & Logging

### Structured Logging

The app uses `tracing` crate for structured logging:

```rust
// In code:
tracing::info!("Event occurred", field = "value");
tracing::warn!("Warning message");
tracing::error!("Error details");
```

### Logging Configuration

```toml
# Cargo.toml (already included)
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

### Log Aggregation

Send logs to:
- **ELK Stack**: Elasticsearch + Kibana
- **Datadog**: `datadog-logs` crate
- **Sentry**: Error tracking
- **CloudWatch**: AWS logging

Example with Datadog:

```rust
use datadog_logs::*;

fn setup_logging() {
    datadog_logs::init()
        .with_service_name("loom-web")
        .with_version("1.0.0")
        .init();
}
```

## Health Checks

### Kubernetes Probes

Already configured in the K8s manifest above:

```yaml
livenessProbe:
  httpGet:
    path: /
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 5
```

### Custom Health Endpoint

Optional: Add a health check endpoint:

```rust
#[server(endpoint = "/health")]
pub async fn health_check() -> Result<String, ServerFnError> {
    Ok("ok".to_string())
}
```

## Security Considerations

### HTTPS/TLS

```nginx
server {
    listen 443 ssl http2;
    server_name loom.example.com;
    
    ssl_certificate /etc/letsencrypt/live/loom.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/loom.example.com/privkey.pem;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
    }
}
```

### CORS Configuration

```rust
// In axum server (if using SSR):
use tower_http::cors::{CorsLayer, Any};

app
    .layer(
        CorsLayer::permissive()
            // Or configure specific origins:
            // .allow_origin("https://trusted.example.com".parse()?)
    )
```

### Content Security Policy

```nginx
add_header Content-Security-Policy "
    default-src 'self';
    script-src 'self' 'wasm-unsafe-eval';
    style-src 'self' 'unsafe-inline';
    connect-src 'self' https://api.loom.example.com;
" always;
```

### Headers Security

```nginx
add_header X-Frame-Options "DENY" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
```

## Rollout & Updates

### Zero-Downtime Deployment

```bash
# 1. Build new version
cargo build --release -p loom-web --features ssr

# 2. Push to registry
docker build -t loom-web:v1.1 .
docker push your-registry/loom-web:v1.1

# 3. Update deployment
kubectl set image deployment/loom-web \
  loom-web=your-registry/loom-web:v1.1

# 4. Monitor rollout
kubectl rollout status deployment/loom-web
kubectl logs -f deployment/loom-web

# 5. Rollback if needed
kubectl rollout undo deployment/loom-web
```

### Canary Deployment

```yaml
# Deploy to 10% of traffic first
apiVersion: v1
kind: Service
metadata:
  name: loom-web-canary
spec:
  selector:
    app: loom-web
    version: canary
  ports:
  - port: 3000
```

## Scaling

### Horizontal Scaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: loom-web-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: loom-web
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

### Load Testing

```bash
# Using Apache Bench
ab -n 10000 -c 100 https://loom.example.com/

# Using wrk
wrk -t12 -c400 -d30s https://loom.example.com/

# Using k6
k6 run load-test.js
```

Load test script (`load-test.js`):
```javascript
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  vus: 50,
  duration: '30s',
};

export default function() {
  let res = http.get('https://loom.example.com/');
  check(res, {
    'status is 200': (r) => r.status === 200,
    'page loads in < 1s': (r) => r.timings.duration < 1000,
  });
}
```

## Troubleshooting

### Issue: WASM module fails to load

**Solution**:
- Ensure MIME type is set: `application/wasm`
- Check browser console for specific error
- Verify CORS headers if loading from CDN

### Issue: High memory usage

**Solution**:
- Profile with `perf`: `cargo flamegraph -p loom-web`
- Check for memory leaks in streaming connections
- Reduce WASM optimization level if compilation hangs

### Issue: Slow initial load

**Solution**:
- Enable gzip/brotli compression
- Use CDN for static assets
- Consider SSR for faster FCP (First Contentful Paint)
- Optimize images and assets

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| WASM bundle size | < 2 MB | ~1-2 MB (est.) |
| Initial load | < 2s | Testing |
| Time to Interactive | < 3s | Testing |
| Component render | < 16ms | Testing |
| Streaming latency | < 100ms | Testing |

## Deployment Checklist

- [ ] Leptos 0.7 migration complete
- [ ] All tests passing (`cargo test -p loom-web`)
- [ ] All clippy warnings resolved
- [ ] Documentation updated
- [ ] Environment variables configured
- [ ] HTTPS/TLS certificates ready
- [ ] Logging aggregation set up
- [ ] Monitoring alerts configured
- [ ] Backup/disaster recovery plan in place
- [ ] Load testing completed
- [ ] Security audit passed
- [ ] Team trained on deployment process
- [ ] Runbooks created for common issues
- [ ] Rollback procedure documented

## Post-Deployment

1. **Monitor metrics** for 24 hours
2. **Check error logs** for any issues
3. **Verify API connectivity** between web and backend
4. **Test key user flows** manually
5. **Collect feedback** from early users
6. **Performance profiling** with real users

## References

- [Leptos Deployment Guide](https://book.leptos.dev/16_global_state.html)
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Nginx Configuration](https://nginx.org/en/docs/)
- [Web Performance Checklist](https://web.dev/lighthouse-performance/)

---

**For support or questions**: Create an issue in the GitHub repository.
