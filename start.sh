#!/bin/sh
set -eu

server &
PORT=3000 node /app/client/build/index.js &
exec caddy run --config /etc/caddy/Caddyfile --adapter caddyfile
