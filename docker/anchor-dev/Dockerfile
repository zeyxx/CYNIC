# CYNIC Anchor Development Environment
#
# Build and deploy Solana programs without Windows headaches
#
# Usage:
#   docker build -t cynic-anchor-dev -f docker/anchor-dev/Dockerfile .
#   docker run -it -v $(pwd):/workspace cynic-anchor-dev
#
# "φ distrusts φ" - κυνικός

FROM ubuntu:24.04

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libudev-dev \
    llvm \
    libclang-dev \
    protobuf-compiler \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install specific Rust version for Solana compatibility
RUN rustup install 1.85.0 && rustup default 1.85.0

# Install Solana CLI
RUN sh -c "$(curl -sSfL https://release.anza.xyz/v2.1.15/install)"
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Install Anchor via AVM
RUN cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
RUN avm install 0.32.1 && avm use 0.32.1

# Install Node.js (for Anchor tests)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
