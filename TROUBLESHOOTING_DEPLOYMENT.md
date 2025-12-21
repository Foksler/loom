# Troubleshooting Deployment Issues

Complete troubleshooting guide for common Loom deployment problems.

## Deployment Script Issues

### Issue: Build Script Fails - Rust Not Found

**Symptoms:**
```
error: command not found: cargo
```

**Solutions:**
1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. Verify installation:
```bash
rustc --version
cargo --version
```

3. Update toolchain:
```bash
rustup update
```

### Issue: Clippy Validation Fails

**Symptoms:**
```
error: use of a disallowed unstable feature
```

**Solutions:**
1. Run clippy to see detailed errors:
```bash
cargo clippy --workspace -- -D warnings
```

2. Auto-fix issues:
```bash
cargo clippy --workspace --fix --allow-dirty --allow-staged
cargo fmt --all
```

3. Run build script with stricter checks:
```bash
./deploy/build.sh release 2>&1 | tee build.log
```

### Issue: Deployment Script Can't Connect via SSH

**Symptoms:**
```
ssh: connect to host prod.example.com port 22: Connection timed out
```

**Solutions:**
1. Test SSH connectivity:
```bash
ssh -vv deploy@prod.example.com "echo 'Connected'"
```

2. Check SSH configuration:
```bash
# Verify SSH key permissions
ls -la ~/.ssh/
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub

# Test with explicit key
ssh -i ~/.ssh/id_rsa deploy@prod.example.com
```

3. Check firewall:
```bash
# From deployment machine
telnet prod.example.com 22

# On target server
sudo ufw allow 22/tcp
sudo firewall-cmd --add-service=ssh
```

4. Verify target server is running:
```bash
ping prod.example.com
nslookup prod.example.com
```

### Issue: Health Check Script Never Passes

**Symptoms:**
```
[WARN] Health check failed (HTTP 503) - attempt 3/5
```

**Solutions:**
1. Check service is actually running:
```bash
# On target server
docker ps | grep loom-server
systemctl status loom
```

2. Verify health endpoint:
```bash
curl -v http://localhost:8080/health
curl -v https://prod.example.com/health
```

3. Check logs:
```bash
docker logs loom-server -n 50
journalctl -u loom -n 50
```

4. Increase timeout and retries:
```bash
./deploy/health-check.sh \
  --url http://prod.example.com:8080 \
  --timeout 30 \
  --retries 10 \
  --interval 10 \
  --verbose
```

5. Run in verbose mode:
```bash
./deploy/health-check.sh --verbose
```

## Service Startup Issues

### Issue: Loom Server Won't Start

**Symptoms:**
- `docker logs loom-server` shows error immediately
- `systemctl status loom` shows failed
- Port already in use error

**Solutions:**
1. Check for port conflicts:
```bash
# Find process using port 8080
lsof -i :8080
netstat -tlnp | grep 8080

# Kill conflicting process
kill -9 <PID>
```

2. Check logs for errors:
```bash
# Docker
docker logs loom-server -f

# Systemd
journalctl -u loom -n 100 -f

# Direct output
/opt/loom/loom-server --log-level debug
```

3. Verify configuration:
```bash
# Check environment variables
env | grep LOOM
cat /etc/loom/.env | head

# Validate syntax
cat /etc/loom/.env | grep -v '^#' | grep -v '^$'
```

4. Check dependencies:
```bash
# Database connectivity
psql -h postgres-host -U loom -d loom -c "SELECT 1"

# Redis connectivity
redis-cli -h redis-host ping

# File permissions
ls -la /var/lib/loom/
ls -la /var/log/loom/
```

### Issue: Out of Memory Error

**Symptoms:**
```
error: cannot allocate memory
killed -9 <PID>
```

**Solutions:**
1. Check memory usage:
```bash
# Current usage
free -h
ps aux | grep loom

# Docker stats
docker stats loom-server
```

2. Increase container memory limit:
```yaml
# docker-compose.yml
services:
  loom-server:
    deploy:
      resources:
        limits:
          memory: 2G
        reservations:
          memory: 1G
```

3. Reduce caching or enable GC:
```bash
# Environment variable to control memory
export MALLOC_TRIM_THRESHOLD_=128000  # Force GC
export MALLOC_MMAP_MAX_=65536
```

4. Monitor memory growth:
```bash
# Watch memory over time
watch -n 1 'ps aux | grep loom-server | grep -v grep'

# Check for memory leaks
valgrind --leak-check=full ./target/release/loom-server
```

## Network & Connectivity Issues

### Issue: Cannot Reach Service from Outside

**Symptoms:**
```
curl: (7) Failed to connect to prod.example.com port 443
Connection refused
```

**Solutions:**
1. Verify service is listening:
```bash
# On server
netstat -tlnp | grep 8080
ss -tlnp | grep 8080
```

2. Check firewall rules:
```bash
# UFW
sudo ufw status
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# firewalld
sudo firewall-cmd --list-all
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload

# iptables
sudo iptables -L -n | grep 80
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
```

3. Check cloud security groups (AWS/Azure/GCP):
```bash
# AWS
aws ec2 describe-security-groups --group-ids sg-xxxxx

# Azure
az network nsg rule list --resource-group mygroup --nsg-name myNSG
```

4. Verify reverse proxy:
```bash
# Test nginx directly
curl -v http://127.0.0.1
nginx -t

# Test upstream
curl -v http://localhost:8080
```

### Issue: TLS/SSL Certificate Errors

**Symptoms:**
```
curl: (60) SSL certificate problem: self signed certificate
```

**Solutions:**
1. Check certificate validity:
```bash
# View certificate details
openssl x509 -in /etc/loom/tls/cert.pem -text -noout

# Check expiration
openssl x509 -in /etc/loom/tls/cert.pem -noout -dates

# Verify chain
openssl verify -CAfile /etc/loom/tls/ca.pem /etc/loom/tls/cert.pem
```

2. Renew certificate (Let's Encrypt):
```bash
# Using Certbot
sudo certbot renew
sudo systemctl restart nginx

# Manual renewal
certbot certonly --standalone -d prod.example.com --force-renewal
```

3. Install certificate:
```bash
# Copy certificate
sudo cp /path/to/cert.pem /etc/loom/tls/
sudo cp /path/to/key.pem /etc/loom/tls/
sudo chown root:root /etc/loom/tls/*
sudo chmod 600 /etc/loom/tls/key.pem
sudo chmod 644 /etc/loom/tls/cert.pem

# Restart services
sudo systemctl restart nginx
sudo systemctl restart loom
```

4. Test SSL configuration:
```bash
# Check SSL strength
openssl s_client -connect prod.example.com:443 -tls1_2

# Scan for vulnerabilities
nmap --script ssl-enum-ciphers -p 443 prod.example.com
```

## Database Issues

### Issue: Cannot Connect to PostgreSQL

**Symptoms:**
```
error: could not connect to server: Connection refused
```

**Solutions:**
1. Check PostgreSQL status:
```bash
# Docker
docker ps | grep postgres
docker logs loom-postgres

# Systemd
systemctl status postgresql
journalctl -u postgresql -n 50
```

2. Test connection:
```bash
# Using psql
psql -h localhost -U loom -d loom -c "SELECT 1"

# From application
curl -X POST http://localhost:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"query":"SELECT 1"}'
```

3. Check PostgreSQL configuration:
```bash
# Inside PostgreSQL container
docker exec -it loom-postgres psql -U loom -c "SHOW listen_addresses;"
docker exec -it loom-postgres cat /var/lib/postgresql/data/postgresql.conf | grep listen_addresses

# Verify port
netstat -tlnp | grep 5432
```

4. Verify credentials:
```bash
# Test with password
psql -h localhost -U loom -d loom -W

# Check user permissions
psql -h localhost -U postgres -d postgres -c "\du"
```

### Issue: Database Schema Not Applied

**Symptoms:**
```
error: relation "users" does not exist
```

**Solutions:**
1. Check migration status:
```bash
# List applied migrations
psql -h localhost -U loom -d loom -c "SELECT * FROM schema_migrations ORDER BY installed_on DESC;"
```

2. Apply pending migrations:
```bash
# Using sqlx
sqlx migrate run --database-url postgresql://loom:password@localhost/loom

# Or manually
psql -h localhost -U loom -d loom < migrations/001_init.sql
```

3. View schema:
```bash
# List tables
psql -h localhost -U loom -d loom -c "\dt"

# Show table structure
psql -h localhost -U loom -d loom -c "\d users"
```

### Issue: Database Performance Degradation

**Symptoms:**
- Slow queries
- High CPU usage
- Response times increasing
- Connection pool exhaustion

**Solutions:**
1. Analyze slow queries:
```bash
# Enable query logging
psql -h localhost -U postgres -d postgres -c "ALTER SYSTEM SET log_min_duration_statement = 1000;"
psql -h localhost -U postgres -d postgres -c "SELECT pg_reload_conf();"

# View slow logs
psql -h localhost -U loom -d loom -c "SELECT query, calls, total_time FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;"
```

2. Add indexes:
```bash
# Check for missing indexes
psql -h localhost -U loom -d loom -c "SELECT schemaname, tablename, indexname FROM pg_indexes WHERE schemaname = 'public';"

# Add index
psql -h localhost -U loom -d loom -c "CREATE INDEX idx_users_email ON users(email);"
```

3. Vacuum database:
```bash
# Run vacuum
psql -h localhost -U loom -d loom -c "VACUUM ANALYZE;"

# Check table sizes
psql -h localhost -U loom -d loom -c "SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size FROM pg_tables ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;"
```

4. Increase connection pool:
```bash
# In .env
DATABASE_MAX_CONNECTIONS=50
DATABASE_MIN_CONNECTIONS=10

# Restart service
systemctl restart loom
```

## Cache Issues

### Issue: Redis Connection Fails

**Symptoms:**
```
error: NOAUTH Authentication required
error: Connection refused
```

**Solutions:**
1. Check Redis status:
```bash
# Docker
docker ps | grep redis
docker logs loom-redis

# Systemd
systemctl status redis-server
```

2. Test Redis connection:
```bash
# Using redis-cli
redis-cli ping
redis-cli -h redis-host ping
redis-cli -h redis-host -a password ping

# From application
curl -X GET http://localhost:8080/api/cache/test
```

3. Check Redis configuration:
```bash
# View running config
redis-cli CONFIG GET "*"

# Check requirepass
redis-cli CONFIG GET requirepass

# View memory
redis-cli INFO memory
```

4. Fix authentication:
```bash
# Set password
redis-cli CONFIG SET requirepass "your-password"
redis-cli CONFIG REWRITE

# Or in .env
REDIS_URL=redis://:password@localhost:6379
```

### Issue: Cache Not Working

**Symptoms:**
- No cache hits
- Cache always returns miss
- Stale data served

**Solutions:**
1. Verify cache is enabled:
```bash
# Check configuration
curl http://localhost:8080/api/config | grep -i cache

# Check environment
echo $REDIS_ENABLED
echo $CACHE_TTL
```

2. Inspect cache content:
```bash
# View keys
redis-cli KEYS "*"

# Check key value
redis-cli GET "key-name"

# Check TTL
redis-cli TTL "key-name"
```

3. Clear cache (if needed):
```bash
# Flush all
redis-cli FLUSHALL

# Flush specific DB
redis-cli SELECT 0 && redis-cli FLUSHDB
```

## Monitoring & Logging Issues

### Issue: No Metrics Appearing in Prometheus

**Symptoms:**
- No data in Grafana
- Prometheus says "no data points"
- Scrape failures

**Solutions:**
1. Check Prometheus scrape status:
```bash
# View targets
curl http://localhost:9090/api/v1/targets

# Check specific job
curl "http://localhost:9090/api/v1/targets?match_target={job='loom-server'}"
```

2. Verify metrics endpoint:
```bash
# Direct access
curl http://localhost:9090/metrics

# From Prometheus server
curl http://localhost:9091/metrics
```

3. Check Prometheus configuration:
```bash
# Validate config
promtool check config prometheus.yml

# Check scrape interval
grep scrape_interval prometheus.yml
```

4. View Prometheus logs:
```bash
# Docker
docker logs loom-prometheus

# Systemd
journalctl -u prometheus -n 50
```

### Issue: Logs Not Appearing

**Symptoms:**
- No logs in centralized logging
- Application logs nowhere to be found
- Log rotation not working

**Solutions:**
1. Check application logging:
```bash
# View log file
tail -f /var/log/loom/loom.log

# Check with different levels
grep ERROR /var/log/loom/loom.log
grep WARN /var/log/loom/loom.log
```

2. Verify log rotation:
```bash
# Check logrotate configuration
cat /etc/logrotate.d/loom

# Manually rotate
logrotate -f /etc/logrotate.d/loom

# Check rotation status
ls -lah /var/log/loom/
```

3. Check log shipping:
```bash
# If using centralized logging (e.g., ELK)
# Verify shipper is running
systemctl status filebeat
docker ps | grep filebeat

# Check logs being shipped
curl http://elasticsearch:9200/_cat/indices
curl http://elasticsearch:9200/loom-*/_search?q=*
```

## Performance Issues

### Issue: High CPU Usage

**Symptoms:**
- CPU at 80-100%
- Slow response times
- User complaints

**Solutions:**
1. Identify CPU bottleneck:
```bash
# View process CPU usage
top -b -n 1 | grep loom

# Profile CPU usage
perf record -p $(pgrep loom-server) -F 99 -g -- sleep 30
perf report

# Flame graph
cargo install flamegraph
cargo flamegraph --bin loom-server
```

2. Optimize code:
```bash
# Check for hot functions
rustflags="-C target-cpu=native" cargo build --release

# Profile with different optimization levels
cargo build -C opt-level=3
```

3. Scale horizontally:
```bash
# Add more replicas
docker-compose up -d --scale loom-server=3

# Or in Kubernetes
kubectl scale deployment loom-server --replicas=5
```

### Issue: High Memory Usage

**Symptoms:**
- Memory leak suspected
- OOM killer invoked
- Service crashes

**Solutions:**
1. Monitor memory:
```bash
# Watch memory growth
watch -n 1 'free -h'

# Track specific process
watch -n 1 'ps aux | grep loom-server'

# Memory profiling
heaptrack ./target/release/loom-server
```

2. Find memory leaks:
```bash
# Valgrind
valgrind --leak-check=full --show-leak-kinds=all ./target/release/loom-server

# Using heaptrack
cargo install heaptrack
heaptrack ./target/release/loom-server
heaptrack_gui heaptrack.loom-server.*.gz
```

3. Optimize memory:
```bash
# Reduce buffer sizes
DATABASE_MAX_CONNECTIONS=10

# Limit cache size
REDIS_MAXMEMORY=512mb

# Restart service
systemctl restart loom
```

## Security Issues

### Issue: Suspicious Activity / Security Breach

**Symptoms:**
- Unusual login attempts
- Unexpected API calls
- Data exfiltration suspected

**Solutions:**
1. Immediate response:
```bash
# Stop the service
systemctl stop loom
docker-compose down

# Preserve logs
tar czf incident-logs-$(date +%s).tar.gz /var/log/

# Contact security team
```

2. Investigate:
```bash
# Review access logs
tail -n 1000 /var/log/nginx/access.log | grep -E "401|403|5[0-9]{2}"

# Check for unauthorized access
grep "Failed password" /var/log/auth.log

# Review audit log
tail -f /var/log/loom/loom-audit.log
```

3. Remediate:
```bash
# Rotate all passwords
# Change API keys
# Update firewall rules
# Enable additional logging
# Consider incident response playbook
```

## Escalation Procedures

### When to Escalate

1. **Critical Outage**
   - Service completely down for >15 minutes
   - Data loss suspected
   - Security breach detected

2. **Performance Degradation**
   - Response times >5 seconds (P95)
   - Error rate >5%
   - Throughput dropped >20%

3. **Unresolved Issues**
   - Issue persists after 30 minutes of troubleshooting
   - Root cause unclear
   - External dependency issue

### Escalation Path

```
1. Assigned Engineer (30 min)
2. Team Lead (15 min)
3. DevOps Manager (15 min)
4. Director of Engineering (5 min)
5. Executive (if customer impact)
```

### Contact Information

- **On-Call:** Check /etc/loom/oncall.txt
- **Team Lead:** team-lead@example.com
- **DevOps:** devops@example.com
- **Security:** security@example.com

## Additional Resources

- [Loom Documentation](../README.md)
- [Deployment Guide](./DEPLOYMENT_AUTOMATION.md)
- [Production Checklist](./PRODUCTION_CHECKLIST.md)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Docker Documentation](https://docs.docker.com/)
- [Kubernetes Docs](https://kubernetes.io/docs/)
