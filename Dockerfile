# CYNIC Organism — Unified Docker Architecture
# One image, multiple roles (API, Bot, Worker)

FROM python:3.13-slim

WORKDIR /app

# Install system dependencies for build and runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency definition
COPY pyproject.toml .

# Install dependencies from pyproject.toml
# We use --no-cache-dir to keep the image slim
RUN pip install --no-cache-dir .

# Copy the entire organism code
COPY . .

# Environment setup
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH=/app

# Create non-root user for security (Axiom BURN: Safety first)
RUN useradd -m -u 1000 cynic && \
    chown -R cynic:cynic /app && \
    mkdir -p /home/cynic/.cynic && \
    chown -R cynic:cynic /home/cynic/.cynic

USER cynic

# Expose API and MCP ports
EXPOSE 8765 8766

# Healthcheck based on our unified status router
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8765/ || exit 1

# Default command: Start the API server
CMD ["python", "cynic/api/server.py"]
