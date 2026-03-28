.PHONY: dev test check

dev:
	trap 'kill 0' EXIT; \
	(cd client && npm run dev) & \
	cargo run -p server & \
	wait

test:
	cargo test --workspace
	cd client && npm run test:unit -- --run

check:
	cargo check --workspace
	cd client && npm run check && npm run lint
