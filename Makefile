NODE_VERSION := $(shell sed 's/^v//' client/.nvmrc)

.PHONY: dev test check build format generate

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

format:
	cargo fmt --all
	cd client && npm run format

build:
	docker build --build-arg NODE_VERSION=$(NODE_VERSION) .

generate:
	bash scripts/generate-agents-md.sh
