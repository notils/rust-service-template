# Multi-stage: compile with the Rust toolchain, ship only the binaries.
# The result is tens of megabytes rather than the ~1.5 GB a toolchain image
# would be (docs/development.md).

# Pinned rather than `latest`: edition 2024 needs >= 1.85, and an unpinned base
# means a toolchain bump can break the build with no commit to point at.
ARG RUST_VERSION=1.97
ARG DEBIAN_RELEASE=bookworm

# ── Planner ──────────────────────────────────────────────────────────────────
# Builds a dependency-only manifest so the expensive `cargo build` of hundreds
# of crates is cached and only re-runs when Cargo.toml/Cargo.lock actually change.
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS planner
WORKDIR /build

RUN cargo install cargo-chef --locked --version ^0.1

COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

# ── Builder ──────────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS builder
WORKDIR /build

RUN cargo install cargo-chef --locked --version ^0.1

# Dependencies first, as their own layer.
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY crates crates

# Both binaries: the API, and the migration runner the release command invokes.
RUN cargo build --release --locked -p {{project-name}}-api -p {{project-name}}-migration --bin {{project-name}}-api --bin migrate

# ── Runtime ──────────────────────────────────────────────────────────────────
# Distroless: no shell, no package manager, no libc utilities. Code execution
# finds nothing to pivot with, and there is no `sh` for an injected command to
# reach. It ships ca-certificates, which is all a Postgres TLS handshake needs.
#
# `:nonroot` runs as uid 65532 without a `USER` line or a useradd step.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

COPY --from=builder /build/target/release/{{project-name}}-api /usr/local/bin/{{project-name}}-api
COPY --from=builder /build/target/release/migrate /usr/local/bin/{{project-name}}-migrate

# Matches the app default. Render overrides PORT, which the config reads.
ENV PORT=8080 \
    HOST=0.0.0.0 \
    LOG_FORMAT=json \
    RUST_LOG=info,{{crate_name}}_api=info,{{crate_name}}_db=info
EXPOSE 8080

# Liveness only — deliberately not /health/ready. A readiness probe here would
# let a brief database blip restart otherwise-healthy containers
# (docs/development.md).
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/{{project-name}}-api", "--health-check"]

# Exec form, so the binary is PID 1 and receives SIGTERM directly. The shell
# form would swallow it and graceful shutdown would never run.
ENTRYPOINT ["/usr/local/bin/{{project-name}}-api"]
