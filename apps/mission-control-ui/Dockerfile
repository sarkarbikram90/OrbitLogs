# Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package configurations
COPY apps/mission-control-ui/package*.json ./

# Install dependencies
RUN npm ci

# Copy application source
COPY apps/mission-control-ui/ ./

# Build production client build
RUN npm run build

# Runner stage
FROM nginx:1.25-alpine

# Create non-root user/group for security hardening (UID/GID 10001)
RUN addgroup -g 10001 -S spaceops && adduser -u 10001 -S spaceops -G spaceops

# Copy built files and apply ownership
COPY --from=builder --chown=spaceops:spaceops /app/dist /usr/share/nginx/html

# Copy non-privileged nginx config
COPY apps/mission-control-ui/nginx.conf /etc/nginx/nginx.conf

# Set permissions for nginx files
RUN chown -R spaceops:spaceops /var/cache/nginx /var/log/nginx /etc/nginx

USER spaceops

EXPOSE 3000

ENTRYPOINT ["nginx", "-g", "daemon off;"]
