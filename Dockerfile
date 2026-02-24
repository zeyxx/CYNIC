# CYNIC Python Kernel — Single-Stage Docker Build
# Build: docker build -t cynic:0.1.0 -f cynic/Dockerfile ..
# "φ distrusts φ" — κυνικός

FROM python:3.13-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy pyproject.toml
COPY pyproject.toml .

# Install Python dependencies
RUN pip install --no-cache-dir \
    "pydantic>=2.6.0" \
    "pydantic-settings>=2.2.0" \
    "httpx>=0.27.0" \
    "surrealdb>=0.3.2" \
    "asyncpg>=0.30.0" \
    "fastapi>=0.110.0" \
    "uvicorn[standard]>=0.29.0" \
    "websockets>=10.4" \
    "prometheus-client>=0.24.0" \
    "mcp>=1.5.0" \
    "typer[all]>=0.12.0" \
    "rich>=13.7.0" \
    "anthropic>=0.23.0" \
    "google-generativeai>=0.8.0" \
    "ollama>=0.4.0" \
    "scikit-learn>=1.5.0" \
    "z3-solver>=4.13.0" \
    "networkx>=3.3" \
    "graphviz>=0.20.0" \
    "ruff>=0.4.0" \
    "numpy>=2.0.0" \
    "textual>=0.80.0" \
    "aiofiles>=23.2.1" \
    "aiohttp>=3.9.0" \
    "docker>=7.0.0"

# Copy source code from parent directory
COPY . .

# Create non-root user
RUN useradd -m -u 1000 cynic && \
    chown -R cynic:cynic /app && \
    mkdir -p /home/cynic/.cynic && \
    chown -R cynic:cynic /home/cynic/.cynic
USER cynic

# Environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

# Expose ports
# Main API: 8765
# MCP Server: 8766
EXPOSE 8765 8766

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8765/health')" || exit 1

# Default command
CMD ["python", "-m", "uvicorn", "cynic.api.server:app", "--host", "0.0.0.0", "--port", "8765"]
