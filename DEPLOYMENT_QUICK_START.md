# Loom Web Deployment Quick Start

**Version**: 0.1.0  
**Last Updated**: 2025-12-22  
**Target Audience**: Deployment Engineers, DevOps Team

---

## TL;DR - 5 Minute Deployment

### Docker Deployment (Recommended)
```bash
# 1. Pull the image (1 min)
docker pull ghuntley/loom-web:latest

# 2. Run the container (1 min)
docker run -d \
  --name loom-web \
  -p 3000:3000 \
  -e LOOM_API_BASE_URL=http://loom-api:8000 \
  -e RUST_LOG=info \
  ghuntley/loom-web:latest

# 3. Verify (2 min)
curl http://localhost:3000/health
curl http://localhost:3000/ready
```

### Kubernetes Deployment (Enterprise)
```bash
# 1. Update image in deploy/production.yaml (30 sec)
sed -i 's|loom-web:.*|loom-web:0.1.0|g' deploy/production.yaml

# 2. Apply deployment (2 min)
kubectl apply -f deploy/production.yaml

# 3. Watch rollout (2 min)
kubectl rollout status deployment/loom-web
kubectl logs -f deployment/loom-web
```

### Docker Compose (Development/Staging)
```bash
# 1. Pull latest images
docker-compose pull

# 2. Start services
docker-compose up -d

# 3. Verify
docker-compose ps
curl http://localhost:3000/health
```

---

## Required Configuration

### Minimum Environment Variables
```bash
LOOM_API_BASE_URL=http://loom-api:8000  # Backend API endpoint
LOOM_WEB_PORT=3000                       # Server port
RUST_LOG=info                            # Log level
```

### Optional but Recommended
```bash
LOOM_SESSION_SECRET=<generate-strong-secret>  # Session encryption
LOOM_CORS_ORIGIN=https://yourdomain.com       # CORS origins
LOOM_ENABLE_METRICS=true                      # Prometheus metrics
LOOM_ENV=production                           # Environment name
```

---

## Verify Deployment

### Health Checks
```bash
# Service health
curl http://localhost:3000/health
# Expected: {"status":"ok","version":"0.1.0"}

# Readiness probe
curl http://localhost:3000/ready
# Expected: {"ready":true,"dependencies":{"api":"ok"}}

# Check logs
docker logs loom-web | tail -20
```

### Functional Tests
```bash
# Test homepage loads
curl -I http://localhost:3000/

# Test API endpoint
curl http://localhost:3000/api/threads

# Test WebSocket (requires wscat)
wscat -c ws://localhost:3000/ws
```

---

## Deployment Strategies

### Strategy 1: Blue-Green (Zero Downtime)
```bash
# Step 1: Deploy new version alongside current
kubectl apply -f deploy/green.yaml

# Step 2: Verify new version
kubectl exec deployment/loom-web-green -- curl localhost:3000/health

# Step 3: Switch traffic
kubectl patch service loom-web -p '{"spec":{"selector":{"version":"0.1.0"}}}'

# Step 4: Monitor
kubectl logs -f deployment/loom-web

# Step 5: Decommission old version (after 30 min)
kubectl delete deployment loom-web-blue
```

### Strategy 2: Canary (Risk Mitigation)
```bash
# Step 1: Deploy new version (10% traffic)
kubectl set image deployment/loom-web \
  loom-web=ghuntley/loom-web:0.1.0

# Step 2: Monitor metrics (5-15 minutes)
# Check: error rate, latency, resource usage
kubectl top pods
kubectl logs deployment/loom-web

# Step 3: Increase traffic gradually
for traffic in 25 50 75 100; do
  kubectl patch virtualservice loom-web \
    --type merge -p "{\"spec\":{\"hosts\":[{\"weight\":$traffic}]}}"
  sleep 300  # Wait 5 min between increases
done
```

### Strategy 3: Rolling Update (Default)
```bash
# Automatic rolling update
kubectl set image deployment/loom-web \
  loom-web=ghuntley/loom-web:0.1.0 \
  --record

# Monitor progress
kubectl rollout status deployment/loom-web -w

# Check status
kubectl get pods -l app=loom-web
```

---

## Quick Rollback

If something goes wrong, rollback immediately:

```bash
# Option 1: Kubernetes (30 seconds)
kubectl rollout undo deployment/loom-web

# Option 2: Docker (switch image)
docker service update --image ghuntley/loom-web:backup loom-web

# Option 3: Docker Compose
docker-compose down
docker-compose up -d  # Will pull old image from backup
```

---

## Troubleshooting

### Service Won't Start
```bash
# Check logs
docker logs loom-web

# Check environment variables
docker inspect loom-web | jq .[0].Config.Env

# Verify API is accessible
curl $LOOM_API_BASE_URL/health

# Check port availability
lsof -i :3000
```

### Health Check Failing
```bash
# Test directly
curl -v http://localhost:3000/health

# Check service is running
ps aux | grep loom
systemctl status loom-web

# Check connectivity
netstat -tuln | grep 3000
```

### High Error Rate
```bash
# Check recent logs for errors
docker logs loom-web --since 5m | grep -i error

# Check resource usage
docker stats loom-web

# Check API connectivity
curl -I http://loom-api:8000/health
```

### Slow Performance
```bash
# Check resource limits
docker inspect loom-web | jq .[0].HostConfig

# Monitor CPU/Memory
docker stats loom-web --no-stream

# Check network latency
ping loom-api
```

---

## Performance Expectations

After deployment, you should see:

| Metric | Expected |
|--------|----------|
| Health check response | < 10ms |
| Page load time | < 2 seconds |
| API response time | < 100ms |
| Memory usage | 30-50 MB |
| CPU usage (idle) | < 1% |
| Error rate | < 0.1% |

---

## Common Commands Reference

```bash
# View deployment status
kubectl describe deployment loom-web

# Check pod details
kubectl describe pod loom-web-abc123

# View recent events
kubectl get events --sort-by='.lastTimestamp' | tail -20

# Check resource usage
kubectl top nodes
kubectl top pods

# Scale replicas
kubectl scale deployment loom-web --replicas=5

# View pod logs (last 100 lines)
kubectl logs deployment/loom-web -c loom-web --tail=100

# Tail logs in real-time
kubectl logs -f deployment/loom-web

# Execute command in pod
kubectl exec -it pod/loom-web-abc123 -- ls -la /

# Port forward to local machine
kubectl port-forward svc/loom-web 3000:3000

# Watch deployment rollout
kubectl rollout status deployment/loom-web -w

# Undo last deployment
kubectl rollout undo deployment/loom-web

# Check rollout history
kubectl rollout history deployment/loom-web
```

---

## Pre-Deployment Checklist

Before deploying, verify:
- [ ] Tests passing: `make test`
- [ ] No lint errors: `make lint`
- [ ] Docker image builds: `docker build -t loom-web:0.1.0 .`
- [ ] Environment variables set
- [ ] Backend API running and healthy
- [ ] Load balancer configured
- [ ] SSL certificates valid
- [ ] Monitoring alerts armed
- [ ] Team notified of deployment window
- [ ] Rollback procedure tested

---

## Post-Deployment Checklist

After deploying, verify:
- [ ] All pods running: `kubectl get pods`
- [ ] Health checks passing: `curl /health`
- [ ] No error spikes in logs
- [ ] Performance metrics normal
- [ ] Users reporting no issues
- [ ] Monitoring shows stable metrics
- [ ] Team notified of success

---

## Support Contacts

For deployment issues:
- **On-call**: [Contact Info]
- **Slack**: #deployments
- **Email**: deployments@company.com
- **Runbook**: See DEPLOYMENT_CHECKLIST.md

---

## Additional Resources

- **Full Documentation**: See [DEPLOYMENT_PACKAGE.md](./DEPLOYMENT_PACKAGE.md)
- **Deployment Checklist**: See [DEPLOYMENT_CHECKLIST.md](./DEPLOYMENT_CHECKLIST.md)
- **Release Notes**: See [RELEASE_NOTES.md](./RELEASE_NOTES.md)
- **Performance Baseline**: See [PERFORMANCE_BASELINE.md](./PERFORMANCE_BASELINE.md)
- **Troubleshooting**: See [DEPLOYMENT_PACKAGE.md](./DEPLOYMENT_PACKAGE.md#troubleshooting)

---

**Questions?** Contact the DevOps team or see the full deployment guide.
