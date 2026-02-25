# Discord Bot Service — CYNIC Community Interface
FROM python:3.13-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements
COPY ../requirements.txt .

# Install Python dependencies
RUN pip install --no-cache-dir \
    discord.py>=2.3.0 \
    aiohttp>=3.9.0 \
    python-dotenv>=1.0.0 \
    && pip install --no-cache-dir -r requirements.txt

# Copy bot code
COPY . /app

# Create non-root user
RUN useradd -m -u 1000 discord && chown -R discord:discord /app
USER discord

# Run bot
CMD ["python", "bot.py"]
