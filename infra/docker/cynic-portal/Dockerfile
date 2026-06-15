FROM node:22-alpine AS build

WORKDIR /app
ENV npm_config_cache=/tmp/npm-cache

RUN apk add --no-cache python3 make g++ linux-headers eudev-dev

COPY package.json package-lock.json ./
COPY packages ./packages
RUN npm ci
RUN npm run build -w packages/cynic-ui

FROM nginx:1.27-alpine

COPY infra/docker/cynic-portal/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/packages/cynic-ui/dist /usr/share/nginx/html

EXPOSE 5000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD wget -qO- http://127.0.0.1:5000/healthz >/dev/null || exit 1
