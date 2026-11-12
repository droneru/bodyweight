# Web app image: the axum server plus the built SPA.
# Build context is the repository root, see deploy/docker-compose.yml and `make docker`.

FROM node:24-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-slim-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p bodyweight-server \
    && cp target/release/bodyweight-server /bodyweight-server

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=server /bodyweight-server /app/bodyweight-server
COPY --from=web /src/web/dist /app/web
ENV BODYWEIGHT_LISTEN=0.0.0.0:3000 \
    BODYWEIGHT_WEB_DIR=/app/web
EXPOSE 3000
ENTRYPOINT ["/app/bodyweight-server"]
