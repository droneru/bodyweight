CARGO        ?= cargo
NPM          ?= npm
COMPOSE_DEV  := docker compose -f deploy/docker-compose.dev.yml
COMPOSE_PROD := docker compose -f deploy/docker-compose.yml

# Connection settings of the dev YDB, see `make dev-env`.
DEV_YDB_ENDPOINT := grpc://localhost:2136
DEV_YDB_DATABASE := /local
DEV_ENV := YDB_ENDPOINT=$(DEV_YDB_ENDPOINT) YDB_DATABASE=$(DEV_YDB_DATABASE) YDB_ANONYMOUS_CREDENTIALS=1

.PHONY: help test lint cli web docker deploy dev dev-db dev-env dev-ydb dev-down dev-clean

help:
	@echo "make test       Rust unit tests"
	@echo "make lint       cargo fmt --check and cargo clippy"
	@echo "make cli        Build the CLI (release) into target/release/bodyweight"
	@echo "make web        Build the SPA into web/dist"
	@echo "make docker     Build the web app docker image"
	@echo "make deploy     On the VPS: git pull, rebuild the app and restart it behind caddy (HTTPS)"
	@echo "make dev        Start YDB, the server and the Vite dev server: http://localhost:5173"
	@echo "make dev-db     Only start YDB in docker and apply migrations"
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

web/node_modules: web/package.json web/package-lock.json
	$(NPM) --prefix web ci
	@touch $@

web: web/node_modules
	$(NPM) --prefix web run build

docker:
	$(COMPOSE_PROD) build app

# Run on the VPS from the repository clone. Caddy obtains and renews HTTPS certificates itself.
deploy:
	git pull --ff-only
	$(COMPOSE_PROD) up -d --build --remove-orphans
	$(COMPOSE_PROD) ps

# Runs the server and the Vite dev server in the foreground; Ctrl+C stops both, YDB keeps running.
dev: dev-db web/node_modules
	$(CARGO) build -p bodyweight-server
	@echo
	@echo "Web app: http://localhost:5173  (API: http://localhost:3000/api)"
	@echo
	@trap 'kill $$(jobs -p) 2>/dev/null' INT TERM EXIT; \
	$(DEV_ENV) BODYWEIGHT_DEV_LOGIN=1 target/debug/bodyweight-server & \
	(cd web && exec node_modules/.bin/vite) & \
	wait

dev-db:
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
