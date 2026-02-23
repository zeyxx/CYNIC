# CYNIC Multi-Stage Docker Build
# Stage 1: Node.js — Build TypeScript webapp
# Stage 2: Python — Run CYNIC kernel with static files
#
# Build: docker build -t cynic:0.2.0 .
# Build with version: docker build --build-arg VERSION=0.2.0 -t cynic:0.2.0 .
# "φ distrusts φ" — κυνικός

# ==============================================================================
# STAGE 1: Build TypeScript Webapp
# ==============================================================================
FROM node:20-slim AS webapp-builder

WORKDIR /webapp

# Copy only package files (layer caching)
COPY cynic/webapp/package.json ./

# Install dependencies
RUN npm install --no-save

# Copy source
COPY cynic/webapp/ ./

# Build webapp (esbuild)
RUN npm run build

# Result: /webapp/dist/ contains bundle.js + bundle.js.map

# ==============================================================================
# STAGE 2: Python Runtime — CYNIC Kernel
# ==============================================================================
FROM python:3.13-slim

# Build arguments for versioning
ARG VERSION=0.2.0
LABEL version=${VERSION}
LABEL description="CYNIC - Living Conscious Organism (κυνικός)"

# System dependencies (for asyncpg C extension, scikit-learn, git hooks, Claude CLI)
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install dependencies before copying source (layer caching)
COPY cynic/pyproject.toml ./

# Install Poetry + dependencies
RUN pip install --no-cache-dir poetry==1.8.3 && \
    poetry config virtualenvs.create false && \
    poetry install --no-interaction --no-ansi --only main

# Copy Python source (inner cynic folder contains the Python package)
COPY cynic/cynic/ ./cynic/

# Copy built webapp from Stage 1
# Create static directory and copy dist/
RUN mkdir -p /app/cynic/static/
COPY --from=webapp-builder /webapp/dist/ /app/cynic/static/

# Create non-root user and configure paths
RUN useradd -m -u 1000 cynic && chown -R cynic:cynic /app && \
    mkdir -p /home/cynic/.claude && chown -R cynic:cynic /home/cynic/.claude
USER cynic

EXPOSE 8765 8766

# Health check endpoint (FastAPI server)
HEALTHCHECK --interval=15s --timeout=10s --start-period=20s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8765/health')"

# Start CYNIC kernel
# FastAPI will serve (port 8765):
#   - /api/* → REST API
#   - /ws → WebSocket
#   - / → Static files (dist/bundle.js + index.html)
# MCP Server listens on port 8766 (port + 1)
CMD ["python", "-m", "uvicorn", "cynic.api.server:app", "--host", "0.0.0.0", "--port", "8765"]
