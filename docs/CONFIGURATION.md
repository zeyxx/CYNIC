# CYNIC Configuration Guide

> "φ distrusts φ" - No hardcoded secrets, ever.

## Environment Separation

CYNIC uses environment-based configuration with **no hardcoded fallbacks**.

| Environment | File | Usage |
|-------------|------|-------|
| Development | `.env` | Local Docker Compose |
| Test | `.env.test` | Automated tests |
| Production | Platform vars | Render, Railway, etc. |

## Configuration Files

```
.env                    # Local development (gitignored)
.env.test               # Test environment (gitignored)
.env.example            # Template (committed)
.env.test.example       # Test template (committed)
```

## Environment Variables

### Required for Production

| Variable | Description | Example |
|----------|-------------|---------|
| `NODE_ENV` | Environment mode | `production` |
| `CYNIC_DATABASE_URL` | PostgreSQL connection | `postgresql://user:<password>@host:5432/db` |

### Optional

| Variable | Description | Default |
|----------|-------------|---------|
| `CYNIC_REDIS_URL` | Redis connection | None (uses memory) |
| `MCP_MODE` | Transport mode | `stdio` |
| `PORT` | HTTP port | `3000` |

### Component Variables (Alternative to URL)

Instead of `CYNIC_DATABASE_URL`, you can use:

| Variable | Description | Default |
|----------|-------------|---------|
| `CYNIC_DB_HOST` | Database host | (required) |
| `CYNIC_DB_PASSWORD` | Database password | (required) |
| `CYNIC_DB_PORT` | Database port | `5432` |
| `CYNIC_DB_USER` | Database user | `cynic` |
| `CYNIC_DB_NAME` | Database name | `cynic` |

## Startup Validation

CYNIC validates configuration at startup:

### Production Mode
- **Fails** if `CYNIC_DATABASE_URL` not set
- **Fails** if localhost detected in production
- **Warns** if Redis not configured
- **Warns** if weak passwords detected

### Development Mode
- **Warns** if services not configured
- Continues with in-memory storage

### Test Mode
- No validation required
- Uses in-memory storage by default

## Local Development Setup

### Option 1: Docker Compose (Recommended)

```bash
# Copy example env file
cp .env.example .env

# Edit with your password
nano .env

# Start services
docker-compose up -d
```

### Option 2: Manual Setup

```bash
# Set environment variables
export CYNIC_DATABASE_URL="postgresql://user:<password>@localhost:5432/cynic"
export CYNIC_REDIS_URL="redis://localhost:6379"

# Run server
npm run dev --workspace=@cynic/mcp
```

## Production Deployment (Render)

1. Create services via Blueprint (`render.yaml`)
2. Environment variables are automatically injected
3. Database URL comes from `cynic-db` service
4. Redis URL comes from `cynic-redis` service

### Manual Override

If automatic injection fails, manually set in Render Dashboard:

```
CYNIC_DATABASE_URL = (copy from cynic-db → Connect → Internal URL)
CYNIC_REDIS_URL = (copy from cynic-redis → Connect → Internal URL)
NODE_ENV = production
```

## Security Best Practices

1. **Never commit `.env` files** - They are gitignored
2. **Use strong passwords** - Minimum 16 characters
3. **Rotate credentials** - Change after any exposure
4. **Use platform secrets** - Render Dashboard, GitHub Secrets
5. **Validate on startup** - Production will fail-fast if misconfigured

## Troubleshooting

### "Database not configured"
- Check `CYNIC_DATABASE_URL` is set
- Or check `CYNIC_DB_HOST` + `CYNIC_DB_PASSWORD` are set

### "Production config invalid"
- Ensure `NODE_ENV=production`
- Ensure database URL is not localhost
- Check Render Dashboard for correct env vars

### "WRONGPASS" Redis error
- Redis URL format: `redis://:<password>@host:6379`
- Copy exact URL from Render Dashboard
