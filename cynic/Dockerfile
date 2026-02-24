# CYNIC Python Kernel — Multi-Stage Docker Build
# Build: docker build -t cynic:0.1.0 -f cynic/Dockerfile .
# "φ distrusts φ" — κυνικός

# ==============================================================================
# STAGE 1: Dependencies
# ==============================================================================
FROM python:3.13-slim AS deps

WORKDIR /app

# Install poetry
RUN pip install --no-cache-dir poetry==1.8.3

# Copy only dependency files for layer caching
COPY pyproject.toml ./

# Install dependencies
RUN poetry export -f requirements.txt --without-hashes > requirements.txt || \
    (poetry install --no-interaction --no-ansi && poetry export -f requirements.txt --without-hashes > requirements.txt)

# ==============================================================================
# STAGE 2: Builder
# ==============================================================================
FROM python:3.13-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install poetry
RUN pip install --no-cache-dir poetry==1.8.3

COPY --from=deps /app/requirements.txt .

RUN pip install --no-cache-dir -r requirements.txt

# Copy source code
COPY . .

# ==============================================================================
# STAGE 3: Production
# ==============================================================================
FROM python:3.13-slim AS production

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy installed packages from builder
COPY --from=builder /usr/local/lib/python3.13/site-packages /usr/local/lib/python3.13/site-packages
COPY --from=builder /usr/local/bin /usr/local/bin

# Copy application code
COPY . .

# Create non-root user
RUN useradd -m -u 1000 cynic && chown -R cynic:cynic /app
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
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8765/health')"

# Default command
CMD ["python", "-m", "uvicorn", "cynic.api.server:app", "--host", "0.0.0.0", "--port", "8765"]
