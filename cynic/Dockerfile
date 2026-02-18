# CYNIC Python Kernel — Docker Image
# Build: docker build -t cynic-kernel .
# "φ distrusts φ" — κυνικός

FROM python:3.11-slim

# System dependencies (for asyncpg, tree-sitter, scikit-learn)
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install dependencies before copying source (layer caching)
COPY pyproject.toml ./

# Install Poetry + dependencies
RUN pip install --no-cache-dir poetry==1.8.3 && \
    poetry config virtualenvs.create false && \
    poetry install --no-interaction --no-ansi --only main

# Copy source
COPY cynic/ ./cynic/

# Create non-root user
RUN useradd -m -u 1000 cynic && chown -R cynic:cynic /app
USER cynic

EXPOSE 8000

# Health check endpoint (FastAPI server)
HEALTHCHECK --interval=15s --timeout=10s --start-period=20s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')"

# Start CYNIC kernel
CMD ["python", "-m", "uvicorn", "cynic.api.server:app", "--host", "0.0.0.0", "--port", "8000"]
