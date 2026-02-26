# CYNIC Governance Bot - Deployment Guide

## Overview

This guide covers deploying the CYNIC Governance Bot in different environments:
- **Development**: Docker Compose for local testing
- **Production**: Kubernetes for scalable deployment

## Prerequisites

### All Environments
- Discord bot token from https://discord.com/developers/applications
- Python 3.13+ (for local development)
- Docker & Docker Compose (for containerized deployment)

### Kubernetes Deployment
- Kubernetes cluster (1.21+)
- kubectl configured to access your cluster
- Persistent storage provisioner
- Container registry access

---

## Development - Docker Compose

### Quick Start

1. **Create .env file** with your Discord token:
```bash
cp .env.template .env
# Edit .env and add your DISCORD_TOKEN
```

2. **Start services**:
```bash
docker-compose -f docker-compose.governance.yml up
```

3. **View logs**:
```bash
docker-compose -f docker-compose.governance.yml logs -f governance-bot
```

4. **Stop services**:
```bash
docker-compose -f docker-compose.governance.yml down
```

### Services

- **governance-bot**: Discord bot service
  - Uses SQLite database (auto-created at `/app/data/governance_bot.db`)
  - Backups stored at `/app/backups`
  - Logs at `/app/logs`

### Configuration

All settings are configured via environment variables in docker-compose.yml:
- Database URL: `sqlite:////app/data/governance_bot.db`
- CYNIC endpoints: `http://localhost:8765` and `http://localhost:8766`
- Logging level: `INFO`

### Troubleshooting

**Bot crashes on startup**:
```bash
# Check logs
docker-compose logs governance-bot

# Verify Discord token is set
docker-compose config | grep DISCORD_TOKEN
```

**Database issues**:
```bash
# Reset database
docker volume rm cynic-governance_governance_bot_data
docker-compose up  # Recreates database
```

**Connection issues**:
```bash
# Test CYNIC connectivity
docker-compose exec governance-bot ping localhost:8765
```

---

## Production - Kubernetes

### Prerequisites

1. **Create namespace**:
```bash
kubectl create namespace cynic-governance
```

2. **Create secrets** with sensitive data:
```bash
kubectl create secret generic governance-bot-secrets \
  --from-literal=DISCORD_TOKEN=<your_token> \
  --from-literal=DATABASE_URL=<your_db_url> \
  -n cynic-governance
```

3. **Build and push Docker image**:
```bash
# Build image
docker build -t my-registry/cynic-governance-bot:latest \
  -f docker/Dockerfile.governance .

# Push to registry
docker push my-registry/cynic-governance-bot:latest
```

4. **Update deployment image** in `kubernetes/deployment.yaml`:
```yaml
image: my-registry/cynic-governance-bot:latest
imagePullPolicy: Always
```

### Deployment Steps

1. **Deploy**:
```bash
kubectl apply -f kubernetes/
```

2. **Verify deployment**:
```bash
# Check pods
kubectl get pods -n cynic-governance

# Check events
kubectl describe deployment governance-bot -n cynic-governance

# View logs
kubectl logs -n cynic-governance \
  deployment/governance-bot --follow
```

3. **Check health**:
```bash
# Get pod status
kubectl get pods -n cynic-governance -o wide

# Check readiness
kubectl get pods -n cynic-governance \
  -o jsonpath='{.items[0].status.conditions[?(@.type=="Ready")].status}'
```

### Configuration

Create a `kubernetes-values.yaml` for your environment:
```yaml
# governance-bot configuration
replicaCount: 2

environment: production

database:
  url: postgresql://user:pass@postgres:5432/governance
  poolSize: 10

cynic:
  url: http://cynic-kernel:8765
  mcpEnabled: true

features:
  gasdfEnabled: true
  nearExecutionEnabled: false
  learningLoopEnabled: true

logging:
  level: INFO
  fileEnabled: true
```

Then apply with:
```bash
kubectl apply -f kubernetes/deployment.yaml \
  --kustomize overlays/production/
```

### Scaling

**Manual scaling**:
```bash
# Scale to 3 replicas
kubectl scale deployment governance-bot --replicas=3 \
  -n cynic-governance
```

**Auto-scaling** (configured in deployment.yaml):
- Minimum: 2 replicas
- Maximum: 5 replicas
- Triggers on CPU (70%) or Memory (80%) utilization

### Updates

**Rolling update**:
```bash
# Update image
kubectl set image deployment/governance-bot \
  governance-bot=my-registry/cynic-governance-bot:v1.1 \
  -n cynic-governance

# Watch rollout
kubectl rollout status deployment/governance-bot \
  -n cynic-governance
```

**Rollback** (if needed):
```bash
kubectl rollout undo deployment/governance-bot \
  -n cynic-governance
```

### Monitoring

**View metrics** (if Prometheus is configured):
```bash
# Port-forward to Prometheus
kubectl port-forward -n monitoring svc/prometheus 9090:9090

# Visit http://localhost:9090
```

**Check logs**:
```bash
# Current logs
kubectl logs -n cynic-governance deployment/governance-bot

# Previous logs (if crashed)
kubectl logs -n cynic-governance deployment/governance-bot \
  --previous

# Specific pod
kubectl logs -n cynic-governance pod/governance-bot-xyz --follow
```

**Check events**:
```bash
kubectl get events -n cynic-governance --sort-by='.lastTimestamp'
```

---

## Database Setup

### SQLite (Development)
- Automatically created on first run
- Located at: `/app/data/governance_bot.db`
- Backups: `/app/backups/governance_db_YYYYMMDD_HHMMSS.sqlite`

### PostgreSQL (Production)

1. **Create database**:
```sql
CREATE DATABASE governance;
CREATE USER cynic WITH PASSWORD 'strong_password';
GRANT ALL PRIVILEGES ON DATABASE governance TO cynic;
```

2. **Set DATABASE_URL**:
```
postgresql://cynic:strong_password@postgres-server:5432/governance
```

3. **Initialize schema** (automatic on first run)

### Backups

**Automatic backups** (enabled by default):
- Runs on shutdown if `DATABASE_BACKUP_ENABLED=true`
- Stored in `/app/backups`
- Kept for last 5 rotations (configurable)

**Manual backup**:
```bash
# Docker
docker-compose exec governance-bot \
  python -c "from governance_bot.database import backup_database; import asyncio; asyncio.run(backup_database())"

# Kubernetes
kubectl exec -n cynic-governance deployment/governance-bot -- \
  python -c "from governance_bot.database import backup_database; import asyncio; asyncio.run(backup_database())"
```

**Restore backup**:
```bash
# Docker
docker-compose exec governance-bot \
  python -c "from governance_bot.database import restore_database; import asyncio; asyncio.run(restore_database(Path('/app/backups/governance_db_20240226_120000.sqlite')))"
```

---

## Health Checks

### Database Health
```bash
# Docker
docker-compose exec governance-bot \
  python -c "from governance_bot.database import db_health_check; import asyncio; print(asyncio.run(db_health_check.check_health()))"

# Kubernetes
kubectl exec -n cynic-governance deployment/governance-bot -- \
  python -c "from governance_bot.database import db_health_check; import asyncio; print(asyncio.run(db_health_check.check_health()))"
```

### Data Consistency
```bash
# Docker
docker-compose exec governance-bot \
  python -c "from governance_bot.database import verify_data_consistency; import asyncio; print(asyncio.run(verify_data_consistency()))"
```

### System Health
- **Bot Status**: Discord bot responds to commands
- **Database**: Can execute queries and transactions
- **CYNIC**: Can reach CYNIC at configured URL
- **Features**: All enabled features are functional

---

## Troubleshooting

### Common Issues

**Discord bot not responding**:
- Check Discord token is valid
- Verify bot has permissions in server
- Check bot logs for connection errors

**Database errors**:
- Check database URL is correct
- Verify database server is running
- Check disk space for SQLite files
- Review database logs

**High memory usage**:
- Check connection pool size
- Increase memory limits in K8s
- Monitor active connections

**Slow performance**:
- Check database query logs
- Monitor CYNIC response times
- Verify network connectivity
- Check CPU utilization

### Debug Mode

Enable debug logging:
```bash
# Docker Compose
LOGGING_LEVEL=DEBUG docker-compose up

# Kubernetes
kubectl patch deployment governance-bot -n cynic-governance -p \
  '{"spec":{"template":{"spec":{"containers":[{"name":"governance-bot","env":[{"name":"LOGGING_LEVEL","value":"DEBUG"}]}]}}}}'
```

### Logs

**Docker Compose**:
```bash
docker-compose logs -f --tail=100 governance-bot
```

**Kubernetes**:
```bash
# Real-time
kubectl logs -f -n cynic-governance deployment/governance-bot

# Last 1000 lines
kubectl logs -n cynic-governance deployment/governance-bot --tail=1000

# Previous run
kubectl logs -n cynic-governance deployment/governance-bot --previous

# All pods
kubectl logs -n cynic-governance -l app=governance-bot --all-containers=true
```

---

## Performance Tuning

### Database Connection Pool
- `DATABASE_POOL_SIZE`: Base pool size (default: 5, prod: 10)
- `DATABASE_MAX_OVERFLOW`: Overflow connections (default: 10, prod: 20)
- Adjust based on concurrent proposal/vote load

### Kubernetes Resources
- **CPU Requests**: 250m (minimum needed)
- **CPU Limits**: 500m (prevent runaway)
- **Memory Requests**: 256Mi (minimum needed)
- **Memory Limits**: 512Mi (prevent OOM)

Adjust based on:
- Number of concurrent proposals
- CYNIC judgment frequency
- Vote processing load

### Replicas and HPA
- Start with 2 replicas for HA
- HPA scales to 5 on high load
- Adjust thresholds based on metrics

---

## Security Considerations

### Secrets Management
- Never commit `.env` files to version control
- Use Kubernetes Secrets for production
- Rotate Discord token periodically
- Store database credentials securely

### Network Policy
- Governance bot can only:
  - Communicate within namespace
  - Reach DNS (port 53)
  - Reach internet (ports 80, 443)
- Prometheus can scrape metrics
- No external ingress by default

### Container Security
- Runs as non-root user (UID 1000)
- Read-only filesystem (except volumes)
- No privilege escalation
- All unnecessary capabilities dropped

### Database Security
- Use strong PostgreSQL passwords
- Enable SSL/TLS for database connections
- Restrict database user permissions
- Enable database backups

---

## Monitoring and Alerting

### Metrics Available
- Discord bot health
- Database connection pool usage
- Proposal processing latency
- Vote recording latency
- CYNIC judgment latency
- Error rates and types

### Prometheus Integration
ServiceMonitor configured for Prometheus Operator:
- Endpoint: `/metrics` on port 8000
- Interval: 30s
- Namespace: cynic-governance

### Alerts to Configure
- Pod CrashLoopBackOff
- PVC running out of space
- High error rate (>1%)
- CYNIC unavailable
- Database connection pool exhausted

---

## Support

For issues or questions:
1. Check logs: `kubectl logs -n cynic-governance deployment/governance-bot`
2. Review configuration: `kubectl get configmap -n cynic-governance`
3. Test connectivity: `kubectl exec -n cynic-governance deployment/governance-bot -- curl localhost:8000`
4. Check events: `kubectl get events -n cynic-governance`

---

## Next Steps

1. **Development**: Test locally with Docker Compose
2. **Staging**: Deploy to staging K8s cluster
3. **Production**: Deploy to production with proper secrets and monitoring
4. **Scaling**: Configure HPA based on actual load
5. **Monitoring**: Set up Prometheus and Grafana
6. **Alerting**: Configure alerts for critical issues
