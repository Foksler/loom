# Loom Web Deployment Checklist

**Version**: 0.1.0  
**Release Date**: 2025-12-22  
**Checklist Owner**: DevOps/Release Team

---

## Pre-Deployment Phase (5-7 days before)

### [ ] Code Quality

- [ ] All tests passing
  ```bash
  make test
  ```
- [ ] No compiler warnings
  ```bash
  make lint
  ```
- [ ] Code coverage meets minimum (70%)
  ```bash
  cargo tarpaulin --out Html --output-dir coverage/
  ```
- [ ] Security scan passed
  ```bash
  cargo audit
  cargo deny check
  ```
- [ ] No TODO/FIXME in critical code paths
  ```bash
  grep -r "TODO\|FIXME" src/ --include="*.rs" | grep -v test
  ```

### [ ] Documentation

- [ ] API documentation complete
  ```bash
  cargo doc --no-deps --open
  ```
- [ ] README.md updated with version
- [ ] CHANGELOG.md entries added
- [ ] Deployment guide reviewed
- [ ] Architecture diagrams current
- [ ] API endpoints documented

### [ ] Build Verification

- [ ] Debug build succeeds
  ```bash
  cargo build
  ```
- [ ] Release build succeeds
  ```bash
  cargo build --release
  ```
- [ ] WASM target builds
  ```bash
  cargo build --target wasm32-unknown-unknown --release
  ```
- [ ] All artifacts generated
  - [ ] Binary: `target/release/loom-server`
  - [ ] WASM: `target/site/pkg/`
  - [ ] Assets: `target/site/`

### [ ] Dependency Review

- [ ] No security vulnerabilities
  ```bash
  cargo audit
  ```
- [ ] All dependencies up-to-date
  ```bash
  cargo update --aggressive
  ```
- [ ] License compliance checked
  ```bash
  cargo license
  ```
- [ ] MSRV (Minimum Supported Rust Version) verified

### [ ] Performance Baseline

- [ ] Build time measured: _____ seconds
- [ ] Bundle size measured: _____ KB
- [ ] Runtime memory baseline: _____ MB
- [ ] Lighthouse score: _____/100
- [ ] No performance regressions from previous version

---

## Build Preparation (3-5 days before)

### [ ] Docker Image Build

- [ ] Dockerfile reviewed
  ```bash
  docker build -f docker/Dockerfile.web -t loom-web:0.1.0 .
  ```
- [ ] Image builds successfully
- [ ] Image size acceptable: < 500 MB
  ```bash
  docker images loom-web:0.1.0
  ```
- [ ] Base image security scanned
  ```bash
  docker scan loom-web:0.1.0
  ```

### [ ] Image Testing

- [ ] Container starts without error
  ```bash
  docker run -d -p 3000:3000 loom-web:0.1.0
  ```
- [ ] Health check passes
  ```bash
  curl http://localhost:3000/health
  curl http://localhost:3000/ready
  ```
- [ ] Volume mounts work correctly
- [ ] Environment variables respected
- [ ] Logs are structured and readable

### [ ] Image Tagging

- [ ] Latest tag applied
  ```bash
  docker tag loom-web:0.1.0 loom-web:latest
  ```
- [ ] Version tag applied
  ```bash
  docker tag loom-web:0.1.0 ghuntley/loom-web:0.1.0
  docker tag loom-web:0.1.0 ghuntley/loom-web:latest
  ```
- [ ] Semver tags applied (major, minor)

### [ ] Image Publishing

- [ ] Authentication configured
  ```bash
  echo $DOCKER_PASSWORD | docker login -u $DOCKER_USERNAME --password-stdin
  ```
- [ ] Image pushed to registry
  ```bash
  docker push ghuntley/loom-web:0.1.0
  docker push ghuntley/loom-web:latest
  ```
- [ ] Tag verification
  ```bash
  docker pull ghuntley/loom-web:0.1.0
  ```

---

## Staging Deployment (2 days before)

### [ ] Environment Preparation

- [ ] Staging environment configured
- [ ] Environment variables set correctly
  - [ ] `LOOM_API_BASE_URL` points to staging API
  - [ ] `RUST_LOG=debug` (for diagnostics)
  - [ ] `LOOM_SESSION_SECRET` generated
- [ ] Database migrations applied (if needed)
- [ ] SSL certificates valid
  ```bash
  openssl x509 -in /path/to/cert -text -noout | grep -A2 "Validity"
  ```

### [ ] Staging Deployment

- [ ] Pull latest image
  ```bash
  docker pull ghuntley/loom-web:0.1.0
  ```
- [ ] Deploy to staging cluster
  ```bash
  kubectl apply -f deploy/staging.yaml
  ```
- [ ] Wait for rollout to complete
  ```bash
  kubectl rollout status deployment/loom-web-staging
  ```
- [ ] Verify pod health
  ```bash
  kubectl get pods -l app=loom-web-staging
  ```

### [ ] Smoke Tests

- [ ] Service responds to health checks
  ```bash
  curl -v https://staging-loom.example.com/health
  ```
- [ ] API endpoints responding
  ```bash
  curl -v https://staging-loom.example.com/api/threads
  ```
- [ ] WebSocket streaming works
  ```bash
  wscat -c wss://staging-loom.example.com/ws
  ```
- [ ] Static assets loading
  ```bash
  curl -v https://staging-loom.example.com/
  ```

### [ ] Functional Testing

- [ ] User login works
- [ ] Thread list displays
- [ ] Search functionality works
- [ ] Code blocks render correctly
- [ ] Message streaming works
- [ ] Navigation between pages smooth
- [ ] Dark mode toggle works
- [ ] Responsive design verified

### [ ] Performance Testing

- [ ] Page load time acceptable
  ```bash
  chrome-devtools-protocol --url=https://staging-loom.example.com
  ```
- [ ] No error logs
  ```bash
  kubectl logs -l app=loom-web-staging --tail=100 | grep -i error
  ```
- [ ] Memory usage stable
  ```bash
  kubectl top pods -l app=loom-web-staging
  ```
- [ ] CPU usage reasonable

### [ ] Security Testing

- [ ] SSL/TLS working
  ```bash
  nmap --script ssl-enum-ciphers https://staging-loom.example.com
  ```
- [ ] Security headers present
  ```bash
  curl -I https://staging-loom.example.com | grep -i "content-security"
  ```
- [ ] CORS properly configured
- [ ] XSS prevention working
- [ ] CSRF token validation working

### [ ] Monitoring Setup

- [ ] Prometheus scraping metrics
- [ ] Grafana dashboards created
- [ ] Alert rules deployed
- [ ] Log aggregation pipeline running
- [ ] APM instrumentation working

---

## Production Deployment (Day 0)

### [ ] Pre-Deployment Backup

- [ ] Current production version backed up
  ```bash
  docker tag ghuntley/loom-web:latest ghuntley/loom-web:backup-$(date +%s)
  ```
- [ ] Database backup created
  ```bash
  kubectl exec -it loom-db -- pg_dump -U postgres > backup.sql
  ```
- [ ] Configuration backed up
  ```bash
  tar -czf loom-web-config-backup.tar.gz /etc/loom-web/
  ```
- [ ] Backups verified
  ```bash
  ls -lh backup*
  ```

### [ ] Production Deployment Strategy

#### [ ] Blue-Green Deployment
- [ ] Blue (current) environment stable
- [ ] Green (new) environment ready
- [ ] Traffic switch plan documented
- [ ] Rollback procedure tested

OR

#### [ ] Canary Deployment
- [ ] Initial 10% traffic to new version
  ```bash
  kubectl patch service loom-web -p '{"spec":{"selector":{"version":"0.1.0"}}}'
  ```
- [ ] Monitor metrics for 15 minutes
- [ ] Gradually increase to 50%
- [ ] Final migration to 100%

OR

#### [ ] Rolling Update
- [ ] Max unavailable pods: 1
  ```bash
  kubectl set image deployment/loom-web loom-web=ghuntley/loom-web:0.1.0
  ```
- [ ] Monitor each pod's health
- [ ] Readiness probes passing

### [ ] Deployment Execution

- [ ] Select deployment window (low-traffic time)
  - Recommended: **Weekday 2-4 AM UTC**
  - Duration: **5-10 minutes**

- [ ] Notify stakeholders
  - [ ] Send deployment notice email
  - [ ] Post in Slack #deployments
  - [ ] Update status page

- [ ] Execute deployment
  ```bash
  kubectl rollout restart deployment/loom-web
  kubectl get rollout status deployment/loom-web
  ```

- [ ] Monitor closely
  - [ ] Watch pod startup logs
  - [ ] Check error rate trending
  - [ ] Monitor resource usage

---

## Post-Deployment Phase

### [ ] Immediate Verification (0-15 minutes)

- [ ] All pods running
  ```bash
  kubectl get pods -l app=loom-web
  ```
- [ ] No pending restarts
  ```bash
  kubectl describe pods -l app=loom-web | grep -i "restart\|error"
  ```
- [ ] Health checks passing
  ```bash
  for i in {1..10}; do curl https://loom.example.com/health; done
  ```
- [ ] API endpoints responding
  ```bash
  curl https://loom.example.com/api/health
  ```
- [ ] No elevated error rates
  ```bash
  kubectl logs -l app=loom-web --tail=50 | grep -i error
  ```

### [ ] Short-term Monitoring (15-60 minutes)

- [ ] Memory usage stable
  ```bash
  watch -n 5 'kubectl top pods -l app=loom-web'
  ```
- [ ] CPU usage reasonable
- [ ] Request latency acceptable
- [ ] Error logs minimal
- [ ] Zero panic messages
- [ ] Performance metrics normal

### [ ] Smoke Tests

- [ ] Homepage loads quickly
  ```bash
  curl -w "Time: %{time_total}s\n" https://loom.example.com/
  ```
- [ ] User can login
- [ ] Thread list displays
- [ ] Search returns results
- [ ] Message streaming works
- [ ] Code blocks render
- [ ] Responsive design works

### [ ] Data Integrity

- [ ] No corrupted data
- [ ] Database indexes intact
- [ ] Cache consistency verified
- [ ] User sessions preserved (if applicable)

### [ ] Notification

- [ ] Update status page to "Operational"
- [ ] Send deployment success email
- [ ] Post success message in Slack #deployments
- [ ] Close deployment ticket

---

## Continued Monitoring (1-7 days)

### [ ] Daily Checks

- [ ] Monitor error rates trend
  ```bash
  prometheus_api GET query='rate(http_requests_total{status=~"5.."}[5m])'
  ```
- [ ] Check latency percentiles
  ```bash
  prometheus_api GET query='histogram_quantile(0.95, http_request_duration_seconds)'
  ```
- [ ] Review application logs
  ```bash
  kubectl logs -l app=loom-web --tail=1000 | tail -100
  ```
- [ ] Verify disk usage stable
  ```bash
  kubectl exec -it loom-web-0 -- df -h
  ```

### [ ] Performance Trending

- [ ] LCP within target
- [ ] First Input Delay acceptable
- [ ] Cumulative Layout Shift minimal
- [ ] Time to Interactive reasonable

### [ ] User Feedback

- [ ] No reported issues
- [ ] Performance acceptable
- [ ] Feature functionality verified
- [ ] UI displays correctly

### [ ] Resource Usage

- [ ] Memory growth normal (not leaking)
- [ ] CPU usage patterns as expected
- [ ] Network bandwidth reasonable
- [ ] Disk I/O acceptable

---

## Rollback Procedures

### Immediate Rollback Trigger
If ANY of the following occurs:
- [ ] Error rate > 5%
- [ ] P95 latency > 2 seconds
- [ ] Service unavailability
- [ ] Data corruption detected
- [ ] Security incident

### Rollback Steps

1. **Pause Traffic** (10 seconds)
   ```bash
   kubectl scale deployment loom-web --replicas=0
   ```

2. **Revert to Previous Image** (30 seconds)
   ```bash
   kubectl set image deployment/loom-web \
     loom-web=ghuntley/loom-web:backup-XXXXX
   ```

3. **Scale Up** (1-2 minutes)
   ```bash
   kubectl scale deployment loom-web --replicas=3
   kubectl rollout status deployment/loom-web
   ```

4. **Verify** (2-5 minutes)
   ```bash
   # Run smoke tests
   ./scripts/smoke-tests.sh
   ```

5. **Notify** (Immediate)
   - Post in Slack #incidents
   - Send incident notification
   - Schedule post-mortem

**Rollback SLA**: Complete rollback within 5 minutes

---

## Post-Deployment Documentation

### [ ] Deployment Report

Create `DEPLOYMENT_REPORT_0.1.0.md`:
```markdown
# Deployment Report - v0.1.0

**Date**: 2025-12-22
**Deployed By**: [Name]
**Duration**: X minutes

## Deployment Details
- Previous version: X
- New version: 0.1.0
- Deployment strategy: [Blue-green/Canary/Rolling]
- Instances updated: X

## Results
- ✓ All pods healthy
- ✓ No errors reported
- ✓ Performance: Baseline met
- ✓ Users: No issues reported

## Metrics
- Deployment time: X minutes
- Downtime: 0 seconds
- Error rate post-deployment: 0.01%
- Success rate: 99.99%
```

### [ ] Lessons Learned

- [ ] Document any issues encountered
- [ ] Note improvements for next deployment
- [ ] Update runbooks if needed
- [ ] Update monitoring alerts

### [ ] Update Documentation

- [ ] Update version in docs
- [ ] Update deployment guide
- [ ] Add release notes to website
- [ ] Update API documentation

---

## Sign-Off

### [ ] Deployment Approval

| Role | Name | Date | Signature |
|------|------|------|-----------|
| DevOps Lead | _____________ | _____ | _____________ |
| Engineering Lead | _____________ | _____ | _____________ |
| Product Manager | _____________ | _____ | _____________ |

### [ ] Post-Deployment Sign-Off

| Role | Name | Date | Verification |
|------|------|------|---------------|
| QA Lead | _____________ | _____ | ✓ All tests passed |
| Operations | _____________ | _____ | ✓ Monitoring active |
| Support | _____________ | _____ | ✓ Ready for users |

---

## Contacts & Escalation

### On-Call Team
- **Primary**: [Name] - [Phone/Slack]
- **Secondary**: [Name] - [Phone/Slack]
- **Manager**: [Name] - [Phone/Slack]

### Emergency Contacts
- **Incident Commander**: [Name]
- **Architecture Lead**: [Name]
- **Database Admin**: [Name]

### Escalation Path
1. On-call engineer
2. Engineering lead
3. Director of Engineering
4. VP of Engineering (if customer impact)

---

## Appendix

### A. Rollback Decision Tree

```
Issue Detected?
  ├─ Error Rate > 5%? → Rollback immediately
  ├─ P95 Latency > 2s? → Rollback immediately
  ├─ Service Down? → Rollback immediately
  ├─ Data Corruption? → Rollback immediately + Incident
  ├─ Minor Issue?
  │  ├─ Can fix quickly? → Deploy hotfix
  │  └─ Complex fix? → Rollback, then fix
  └─ No Issues? → Continue monitoring
```

### B. Communication Templates

#### Deployment Start
```
🚀 Starting deployment of Loom Web v0.1.0
  Strategy: Blue-green
  Window: 2:00 AM UTC
  Expected duration: 10 minutes
  Rollback plan: Ready
```

#### Deployment Complete
```
✅ Deployment of Loom Web v0.1.0 complete
  Status: Successful
  Duration: 8 minutes
  Error rate: 0.01%
  All systems normal
```

#### Rollback Executed
```
⚠️ Rollback initiated for Loom Web
  Issue: [Description]
  Previous version: [Version]
  Status: Rolled back
  Next steps: Incident investigation
```

### C. Useful Commands

```bash
# Check deployment status
kubectl rollout status deployment/loom-web

# View recent changes
kubectl rollout history deployment/loom-web

# Check pod logs
kubectl logs -f deployment/loom-web

# Get resource usage
kubectl top nodes
kubectl top pods

# View events
kubectl get events --sort-by='.lastTimestamp'

# Execute shell in pod
kubectl exec -it pod/loom-web-xxx -- /bin/bash

# Port forward for testing
kubectl port-forward pod/loom-web-xxx 3000:3000

# Scale replicas
kubectl scale deployment/loom-web --replicas=3
```

---

**Last Updated**: 2025-12-22  
**Next Review**: Post-deployment  
**Owner**: DevOps Team
