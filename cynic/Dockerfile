# CYNIC Python Kernel — Docker Image
# Build: docker build -t cynic-kernel .
# "φ distrusts φ" — κυνικός

FROM python:3.13-slim

# System dependencies (for asyncpg C extension, scikit-learn, git hooks, Node.js/npm)
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libpq-dev \
    git \
    curl \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install dependencies before copying source (layer caching)
COPY pyproject.toml ./

# Install Poetry + dependencies
RUN pip install --no-cache-dir poetry==1.8.3 && \
    poetry config virtualenvs.create false && \
    poetry install --no-interaction --no-ansi --only main

# Install Claude CLI globally (for action execution)
RUN npm install -g @anthropic-ai/claude-cli@latest && \
    npm cache clean --force

# Copy source
COPY cynic/ ./cynic/

# Create non-root user and configure for Claude CLI
RUN useradd -m -u 1000 cynic && chown -R cynic:cynic /app && \
    mkdir -p /home/cynic/.claude && chown -R cynic:cynic /home/cynic/.claude
USER cynic

# Configure Claude CLI path (accessible to cynic user)
ENV PATH="/home/cynic/.npm/bin:${PATH}"

EXPOSE 8000

# Health check endpoint (FastAPI server)
HEALTHCHECK --interval=15s --timeout=10s --start-period=20s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')"

# Start CYNIC kernel
CMD ["python", "-m", "uvicorn", "cynic.api.server:app", "--host", "0.0.0.0", "--port", "8000"]
