ARG NODE_VERSION

FROM rust:1 AS rust-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY adapters ./adapters
COPY server ./server
RUN cargo build -p server --release

FROM node:${NODE_VERSION}-slim AS node-builder
WORKDIR /build/client
COPY client/package*.json ./
RUN npm ci
COPY client ./
RUN npm run build

FROM caddy:2 AS caddy

FROM node:${NODE_VERSION}-slim
RUN apt-get update && apt-get install -y --no-install-recommends tini && rm -rf /var/lib/apt/lists/*
COPY --from=caddy /usr/bin/caddy /usr/local/bin/caddy
COPY --from=rust-builder /build/target/release/server /usr/local/bin/server
COPY --from=node-builder /build/client/build /app/client/build
COPY Caddyfile /etc/caddy/Caddyfile
COPY start.sh /app/start.sh
RUN chmod +x /app/start.sh
WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["tini", "--"]
CMD ["/app/start.sh"]
