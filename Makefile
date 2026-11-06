CARGO       ?= cargo
COMPOSE_DEV := docker compose -f deploy/docker-compose.dev.yml

# Connection settings of the dev YDB, see `make dev-env`.
DEV_YDB_ENDPOINT := grpc://localhost:2136
DEV_YDB_DATABASE := /local
DEV_ENV := YDB_ENDPOINT=$(DEV_YDB_ENDPOINT) YDB_DATABASE=$(DEV_YDB_DATABASE) YDB_ANONYMOUS_CREDENTIALS=1

.PHONY: help test lint cli dev dev-env dev-ydb dev-down dev-clean

help:
	@echo "make test       Rust unit tests"
	@echo "make lint       cargo fmt --check and cargo clippy"
	@echo "make cli        Build the CLI (release) into target/release/bodyweight"
	@echo "make dev        Start YDB in docker and apply migrations"
	@echo "make dev-env    Print env variables to connect the CLI to the dev YDB"
	@echo "make dev-ydb    Open the ydb CLI on the dev YDB; pass a command via ARGS"
	@echo "make dev-down   Stop the dev environment, keeping the data"
	@echo "make dev-clean  Stop the dev environment and delete the data"

test:
	$(CARGO) test --workspace

lint:
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings

cli:
	$(CARGO) build --release -p bodyweight-cli
	@echo "Built target/release/bodyweight"

dev:
	$(COMPOSE_DEV) up -d --wait ydb
	$(COMPOSE_DEV) run --rm migrate
	@echo
	@echo "YDB is up: grpc://localhost:2136/local, UI http://localhost:8765"
	@echo "Connect the CLI:  eval \"\$$(make -s dev-env)\"   (fish: make -s dev-env | source)"

dev-env:
	@for kv in $(DEV_ENV); do echo "export $$kv"; done

# Interactive YQL shell by default, e.g. `make dev-ydb ARGS="scheme ls"`.
dev-ydb:
	ydb -e $(DEV_YDB_ENDPOINT) -d $(DEV_YDB_DATABASE) $(ARGS)

dev-down:
	$(COMPOSE_DEV) down

dev-clean:
	$(COMPOSE_DEV) down --volumes
