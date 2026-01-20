# CYNIC MCP Server - Production Dockerfile
#
# Multi-stage build for minimal production image
#
# "φ distrusts φ" - κυνικός

# ==============================================================================
# Stage 1: Dependencies
# ==============================================================================
FROM node:20-slim AS deps

WORKDIR /app

# Copy package files for all workspaces
COPY package.json package-lock.json* ./
COPY packages/core/package.json ./packages/core/
COPY packages/protocol/package.json ./packages/protocol/
COPY packages/node/package.json ./packages/node/
COPY packages/persistence/package.json ./packages/persistence/
COPY packages/holdex/package.json ./packages/holdex/
COPY packages/gasdf/package.json ./packages/gasdf/
COPY packages/zk/package.json ./packages/zk/
COPY packages/anchor/package.json ./packages/anchor/
COPY packages/burns/package.json ./packages/burns/
COPY packages/mcp/package.json ./packages/mcp/

# Install production dependencies only
# npm workspaces hoist all deps to root node_modules
RUN npm ci --omit=dev --workspace=@cynic/mcp

# ==============================================================================
# Stage 2: Production
# ==============================================================================
FROM node:20-slim AS production

# Security: Run as non-root user
RUN groupadd -r cynic && useradd -r -g cynic cynic

WORKDIR /app

# Copy root node_modules (npm workspaces hoists all deps here)
COPY --from=deps /app/node_modules ./node_modules

# Copy source code
COPY package.json ./
COPY packages/core ./packages/core
COPY packages/protocol ./packages/protocol
COPY packages/node ./packages/node
COPY packages/persistence ./packages/persistence
COPY packages/holdex ./packages/holdex
COPY packages/gasdf ./packages/gasdf
COPY packages/zk ./packages/zk
COPY packages/anchor ./packages/anchor
COPY packages/burns ./packages/burns
COPY packages/mcp ./packages/mcp

# Set ownership
RUN chown -R cynic:cynic /app

# Switch to non-root user
USER cynic

# Environment defaults
ENV NODE_ENV=production
ENV MCP_MODE=http
ENV PORT=3000

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD node -e "fetch('http://localhost:3000/health').then(r => r.ok ? process.exit(0) : process.exit(1)).catch(() => process.exit(1))"

# Start server
CMD ["node", "packages/mcp/bin/mcp.js"]
