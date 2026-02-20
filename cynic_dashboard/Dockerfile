# Build stage
FROM node:18-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source
COPY . .

# Build for production
RUN npm run build

# Runtime stage
FROM node:18-alpine

WORKDIR /app

# Install lightweight HTTP server
RUN npm install -g http-server

# Copy built assets from builder
COPY --from=builder /app/dist ./dist

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://localhost:3000 || exit 1

# Expose port
EXPOSE 3000

# Default environment (can be overridden)
ENV VITE_ORGANISM_URL=http://cynic:8000
ENV VITE_WS_URL=ws://cynic:8000

# Start HTTP server serving dist directory
CMD ["http-server", "dist", "-p", "3000", "--cors"]
