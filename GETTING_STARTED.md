# 🎯 Getting Started with Loom

Your first steps to using Loom for distributed tracing.

## 📋 Prerequisites

Before starting, ensure you have:

- ✅ Loom installed ([INSTALLATION.md](file:///home/ghuntley/loom/INSTALLATION.md))
- ✅ Basic understanding of distributed systems
- ✅ Terminal/CLI familiarity
- ✅ Text editor or IDE (optional)

**Verify Installation**:
```bash
loom --version
loom-server --help
```

---

## 🚀 5-Minute Quickstart

### Step 1: Start the Server

```bash
# Start loom-server (runs on http://localhost:8080)
loom-server
```

Expected output:
```
2025-12-22T10:30:45.123Z  INFO loom_server: Server listening on 127.0.0.1:8080
2025-12-22T10:30:45.234Z  INFO loom_server: Web UI available at http://127.0.0.1:8080
```

### Step 2: Access the Web UI

Open your browser and navigate to:
```
http://localhost:8080
```

You should see the Loom dashboard with:
- 📊 Overview panel
- 🔍 Query interface
- 📈 Real-time metrics
- 🎛️ Configuration panel

### Step 3: Create Your First Trace

**Using CLI**:
```bash
# Create a simple trace
loom trace \
  --service "my-app" \
  --span "request-handler" \
  --duration 150ms \
  --status "success"
```

**Using Web UI**:
1. Click "New Trace" button
2. Fill in service name: `my-app`
3. Add span: `request-handler`
4. Set duration: `150ms`
5. Click "Create"

### Step 4: Query Your Traces

**Using CLI**:
```bash
# Find all traces from my-app
loom query --service "my-app"

# Find slow traces (> 100ms)
loom query --service "my-app" --min-duration 100ms
```

**Using Web UI**:
1. Go to "Query" tab
2. Enter service: `my-app`
3. Click "Search"
4. View results in table

### Step 5: View Trace Details

**Using CLI**:
```bash
# Show trace details
loom show <trace-id>
```

**Using Web UI**:
1. Click on a trace in results
2. View spans, timeline, and metadata
3. Analyze performance

---

## 📚 Basic Concepts

### Traces
A trace represents a single request flowing through your system.

```
Trace ID: a1b2c3d4
├── Service: api-gateway
│   └── Span: handle-request [0ms - 150ms]
├── Service: auth-service
│   └── Span: validate-token [50ms - 80ms]
└── Service: database
    └── Span: query-users [100ms - 140ms]
```

### Spans
Individual operations within a trace.

```rust
// Example in your code
let span = tracer.start_span("query-users");
// ... do work ...
span.end();
```

### Attributes
Key-value metadata on spans.

```
Span: query-users
├── db.system: postgres
├── db.statement: SELECT * FROM users
├── http.status_code: 200
└── duration: 40ms
```

---

## 🔧 Configuration

### Server Configuration

**Configuration File** (`~/.config/loom/config.toml`):
```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4
request_timeout = "30s"

[logging]
level = "info"        # debug, info, warn, error
format = "json"       # json or text

[tracing]
buffer_size = 100000
max_trace_size = 100000
retention_days = 7
```

**Environment Variables**:
```bash
# Set log level
export RUST_LOG=debug

# Set buffer size
export LOOM_BUFFER_SIZE=50000

# Set port
export LOOM_PORT=9000
```

**Command-line Options**:
```bash
loom-server \
  --host 0.0.0.0 \
  --port 8080 \
  --workers 4 \
  --log-level debug
```

---

## 💡 Common Tasks

### Task 1: Instrument Your Application

**In Your Rust Code**:
```rust
use tracing::{info, span, Level};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Create a span
    let span = span!(Level::INFO, "process_request");
    let _enter = span.enter();

    info!("Handling request");
    // ... your code ...
    info!("Request complete");
}
```

**Compile and Run**:
```bash
cargo build --release
./target/release/my-app
```

### Task 2: Export Traces to Loom

**Option 1: Direct HTTP Export**
```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::new_exporter;

let exporter = new_exporter()
    .http()
    .endpoint("http://localhost:4318")
    .build_span_exporter()?;

// Use in your tracing setup
```

**Option 2: Using Docker Compose**
See [INSTALLATION.md](file:///home/ghuntley/loom/INSTALLATION.md#docker-compose)

### Task 3: Create Dashboards

**In Web UI**:
1. Go to "Dashboards" section
2. Click "New Dashboard"
3. Add widgets:
   - Trace count by service
   - Average latency
   - Error rate
   - P99 latency
4. Save dashboard

### Task 4: Set Up Alerts

**Alert Configuration**:
```bash
loom alert create \
  --name "high-latency" \
  --condition "p99_latency > 500ms" \
  --action "email:ops@company.com"
```

### Task 5: Integrate with Grafana

**Add Loom Data Source**:
1. Open Grafana
2. Settings → Data Sources → Add
3. Choose "Loom" (if available) or "HTTP API"
4. URL: `http://localhost:8080/api`
5. Save & Test

---

## 🧪 Example: Full Application

Create `main.rs`:
```rust
use tracing::{info, Level, span};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let span = span!(Level::INFO, "application");
    let _guard = span.enter();

    info!("Application started");
    
    // Simulate some work
    simulate_request().await;
    
    info!("Application finished");
}

async fn simulate_request() {
    let span = span!(Level::INFO, "request");
    let _guard = span.enter();
    
    info!("Processing request");
    sleep(Duration::from_millis(100)).await;
    info!("Request complete");
}
```

**Cargo.toml**:
```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt"] }
```

**Build & Run**:
```bash
cargo build
cargo run
```

---

## 🔍 Querying Traces

### Basic Query Syntax

```bash
# By service
loom query --service "my-service"

# By operation
loom query --operation "process-request"

# By time range
loom query --start "2025-12-22T10:00:00" --end "2025-12-22T11:00:00"

# By duration
loom query --min-duration 100ms --max-duration 1s

# By status
loom query --status "error"

# Combine filters
loom query \
  --service "api" \
  --operation "POST /users" \
  --min-duration 200ms \
  --status "error"
```

### Advanced Queries

**Using the Web UI Query Builder**:
1. Click "Advanced Query"
2. Build complex filters:
   ```
   service = "api" AND 
   duration > 200ms AND 
   error = true
   ```
3. Click "Search"

---

## 📊 Monitoring Your System

### Key Metrics to Track

| Metric | Command | Threshold |
|--------|---------|-----------|
| **Request Count** | `loom metrics --metric "request_count"` | Expected rate |
| **Latency (p50)** | `loom metrics --metric "latency_p50"` | < 50ms |
| **Latency (p99)** | `loom metrics --metric "latency_p99"` | < 500ms |
| **Error Rate** | `loom metrics --metric "error_rate"` | < 0.1% |
| **Throughput** | `loom metrics --metric "throughput"` | Expected rps |

### Example Monitoring Script

```bash
#!/bin/bash
# monitor.sh - Simple monitoring script

while true; do
    echo "=== Loom Status ==="
    echo "Current traces: $(loom metrics --metric trace_count)"
    echo "P99 Latency: $(loom metrics --metric latency_p99)"
    echo "Error Rate: $(loom metrics --metric error_rate)"
    echo ""
    sleep 30
done
```

---

## 🚀 Next Steps

### Ready for More?

1. **Learn Architecture**: [COMPONENT_ARCHITECTURE.md](file:///home/ghuntley/loom/COMPONENT_ARCHITECTURE.md)
2. **Advanced Queries**: [QUERY_TRACING_DOCUMENTATION.md](file:///home/ghuntley/loom/QUERY_TRACING_DOCUMENTATION.md)
3. **Deployment**: [DEPLOYMENT_GUIDE.md](file:///home/ghuntley/loom/DEPLOYMENT_GUIDE.md)
4. **Development**: [DEV_ENVIRONMENT_SETUP.md](file:///home/ghuntley/loom/DEV_ENVIRONMENT_SETUP.md)
5. **API Reference**: [API_QUICK_REFERENCE.md](file:///home/ghuntley/loom/API_QUICK_REFERENCE.md)

### Exploration Path

```
Getting Started (you are here)
    ↓
Try Examples
    ↓
Read Architecture
    ↓
Build Something
    ↓
Deploy to Production
    ↓
Monitor & Optimize
```

---

## 🆘 Troubleshooting

### Server Won't Start

```bash
# Check port availability
lsof -i :8080

# Use different port
loom-server --port 8081

# Check logs
RUST_LOG=debug loom-server
```

### Can't Find Traces

```bash
# Verify server is running
curl http://localhost:8080/health

# Check trace export
loom metrics --metric "traces_received"

# View all traces
loom query --service "*"
```

### Connection Refused

```bash
# Verify server address
loom-server --host 0.0.0.0 --port 8080

# From another machine
curl http://<server-ip>:8080
```

### High CPU/Memory Usage

```bash
# Check configuration
loom config show

# Reduce buffer size
export LOOM_BUFFER_SIZE=10000
loom-server

# Check for memory leaks
loom metrics --metric "memory_usage"
```

---

## 📚 Additional Resources

| Topic | Link |
|-------|------|
| **Complete Guide** | [FINAL_DOCUMENTATION_INDEX.md](file:///home/ghuntley/loom/FINAL_DOCUMENTATION_INDEX.md) |
| **API Reference** | [API_QUICK_REFERENCE.md](file:///home/ghuntley/loom/API_QUICK_REFERENCE.md) |
| **Troubleshooting** | [TROUBLESHOOTING_DEPLOYMENT.md](file:///home/ghuntley/loom/TROUBLESHOOTING_DEPLOYMENT.md) |
| **FAQ** | [FAQ_QUERY_BRIDGE.md](file:///home/ghuntley/loom/FAQ_QUERY_BRIDGE.md) |

---

## ✅ Completed Milestones

- [x] Install Loom
- [x] Start server
- [x] Access web UI
- [x] Create first trace
- [x] Query traces
- [x] Configure system
- [x] Understand concepts

**🎉 You're ready to use Loom!**

---

**Last Updated**: 2025-12-22
**For Questions**: See [FINAL_DOCUMENTATION_INDEX.md](file:///home/ghuntley/loom/FINAL_DOCUMENTATION_INDEX.md)
