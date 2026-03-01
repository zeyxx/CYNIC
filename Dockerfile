# BUILD STAGE
FROM python:3.13-slim as builder

WORKDIR /build
COPY pyproject.toml .
RUN pip install --no-cache-dir wheel && \
    pip wheel --no-cache-dir --wheel-dir /build/wheels -r pyproject.toml || \
    pip wheel --no-cache-dir --wheel-dir /build/wheels .

# FINAL STAGE
FROM python:3.13-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r cynic && useradd -r -g cynic cynic
RUN mkdir -p /home/cynic/.cynic && chown -r cynic:cynic /home/cynic

# Copy wheels from builder
COPY --from=builder /build/wheels /wheels
RUN pip install --no-cache-dir /wheels/*

# Copy source code
COPY . .
RUN chown -R cynic:cynic /app

USER cynic

EXPOSE 58765

# SRE: Use standard environment variables
ENV PYTHONUNBUFFERED=1
ENV PORT=58765

CMD ["python", "-m", "cynic.interfaces.api.server"]
