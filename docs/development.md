# Development

| | |
|---|---|
| **Status** | Active |
| **Toolchain** | Rust stable · Postgres 18 · Docker |

---

## Setup

```bash
rustup toolchain install stable
cargo install sea-orm-cli      # optional: regenerating entities by hand still works without it
cargo install cargo-watch      # optional: rebuild + rerun on every source change

cp .env.example .env

docker compose up -d postgres      # start Postgres, wait until it reports healthy
cargo run -p {{project-name}}-migration -- up
cargo run -p {{project-name}}-api
```

Two probes, deliberately separate:

```bash
curl localhost:8080/health         # liveness — process only, no database
curl localhost:8080/health/ready   # readiness — includes a database round-trip
```

⚠️ **Liveness must not touch the database.** If it did, a brief database blip would make the orchestrator kill and restart otherwise-healthy instances. Readiness is the one that gates traffic; it returns `503` when Postgres is unreachable.

`PORT` overrides the default `8080`.

### Auto-reload

```bash
cargo watch -q -c -w crates/ -x 'run -p {{project-name}}-api'
```

`-q` silences cargo-watch's own output, `-c` clears the screen between runs, `-w` picks the directory to watch, `-x` is the cargo subcommand to re-run.

⚠️ **Pin the local Postgres major version to whatever your production Postgres runs.** A migration that passes on 18 and fails on 17 is a bad way to discover a version mismatch.

The volume is mounted at `/var/lib/postgresql`, the parent, following the [official image guidance](https://hub.docker.com/_/postgres) for PG 18 and above — `PGDATA` is version-specific since 18, so mounting the parent keeps each major version's data directory in the same volume, which is what allows `pg_upgrade` to run against the old and new directories side by side.

---

## Environment

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | Postgres connection string |
| `HOST` / `PORT` | Listen address. Default `0.0.0.0:8080` |
| `DATABASE_MAX_CONNECTIONS` | Pool size per instance. Default `10` |
| `DATABASE_LOG_STATEMENTS` | Logs every SQL statement. Local debugging only — statements can carry personal data |
| `REQUEST_TIMEOUT_SECONDS` | Default `30` |
| `LOG_FORMAT` | `json` in deployment; human-readable otherwise |

Add your own as your service needs them, in `crates/{{project-name}}-api/src/config.rs`.

Startup validation collects **every** problem before exiting, so a fresh checkout reports all missing variables at once. A variable that is present but unparseable is an error rather than a silent fallback.

🔴 **Never commit `.env` or any key material.** `.env.example` documents names with placeholder values only.

---

## Layout

```
crates/
├── {{project-name}}-types/     shared types, error enums — no dependencies
├── {{project-name}}-core/      domain logic — NO I/O, unit-testable without a DB
├── {{project-name}}-db/        SeaORM entities, repositories, connection pool
├── {{project-name}}-migration/ schema migrations + the migrate CLI
└── {{project-name}}-api/       axum routes, middleware
```

🔴 **`{{project-name}}-core` must not import `sea-orm`, `reqwest`, or `tokio::net`.** If logic seems to need I/O, it belongs in `{{project-name}}-db` or `{{project-name}}-api` — pass data in, return decisions out. This is what keeps domain logic testable without fixtures.

Check the rule rather than trusting it:

```bash
cargo tree -p {{project-name}}-core -e normal   # must show no sea-orm, reqwest, or tokio
```

CI's `no-io-in-core` job enforces this mechanically on every push, not just as a convention.

---

## API docs

`crates/{{project-name}}-api/src/routes/mod.rs` builds the router through `utoipa_axum::OpenApiRouter` rather than a plain `axum::Router`, so every route doubles as an OpenAPI path — there's no separate spec to hand-maintain. Served at `/docs` (Swagger UI) and `/api-docs/openapi.json` (raw spec, importable into Postman).

**Adding a route:**

1. Derive `utoipa::ToSchema` on any new request/response struct, alongside its existing `Serialize`/`Deserialize`.
2. Annotate the handler with `#[utoipa::path(...)]` — method, `path` (relative to wherever it's nested, not the full URL), `tag`, `request_body` if any, every `responses(...)` entry the handler can actually return.
3. Register it with `.routes(utoipa_axum::routes!(handler))` in `routes/mod.rs`, not axum's own `.route(path, method(handler))` — the latter compiles fine but silently produces a route `/docs` never learns about.

⚠️ **`path` inside `#[utoipa::path]` is relative to its nesting, and it is also the literal route axum registers — not just documentation metadata.** If you nest a route table under a prefix (e.g. `.nest("/v1", v1())`), a handler inside it must use a path relative to that prefix, not the full path — getting this wrong double-prefixes the *real* route, not just the doc. Verify with a real request after adding a route, not just by reading the generated spec.

---

## Migrations

```bash
cargo run -p {{project-name}}-migration -- up          # apply all pending
cargo run -p {{project-name}}-migration -- down -n 1   # revert the last one
cargo run -p {{project-name}}-migration -- status
cargo run -p {{project-name}}-migration -- fresh       # drop and re-apply (local only)
```

`m20260101_000001_create_example_table` is a real, working example migration — copy its shape for your first real one, then delete it (and its entity, once you add one, in `{{project-name}}-db`).

```bash
# regenerate entities after a schema change — commit the result
sea-orm-cli generate entity -o crates/{{project-name}}-db/src/entities
```

⚠️ Hand-write entity doc comments explaining *why* a column is shaped the way it is — a `sea-orm-cli generate entity` re-run discards them. Regenerate into a scratch location and hand-port the shape back if you need to.

**Verify a migration both ways before committing.** `down` is only known to work once it has run:

```bash
cargo run -p {{project-name}}-migration -- down -n 1 && cargo run -p {{project-name}}-migration -- up
```

**Rules:**

- Forward-only in production — never edit an applied migration
- Never ship a destructive migration in the same release as the code needing it: deploy the additive change, migrate data, remove the old column in a later release
- Run on staging before production, always

---

## Testing

```bash
cargo test -p {{project-name}}-core            # fast, no database
cargo test                                     # full suite (needs Postgres)
cargo clippy --all-targets -- -D warnings
cargo audit
```

Most domain-logic tests belong in `{{project-name}}-core` and need no database — deliberately. Repository tests that need real Postgres behaviour (constraints, joins) live in `{{project-name}}-db`, sharing the `test_support::db()` helper in its `lib.rs`.

⚠️ **`cargo audit` from the first commit.** A dependency advisory is not a routine upgrade to defer.

---

## CI

Defined in [`ci.yml`](../.github/workflows/ci.yml) — see [Deploying](#deploying) for the job breakdown.

---

## Deploying

See [docs/deployment.md](deployment.md) for the full runbook.

### The image

Multi-stage build, defined in [`Dockerfile`](../Dockerfile) (`cargo-chef` + distroless).

```bash
docker build -t {{project-name}} .
docker run --rm -p 8080:8080 \
  -e DATABASE_URL="postgres://..." \
  {{project-name}}
```

| Choice | Why |
|---|---|
| `gcr.io/distroless/cc-debian12:nonroot` runtime | No shell, no package manager. Code execution finds nothing to pivot with, and there is no `sh` for an injected command to reach |
| `cargo-chef` dependency layer | Hundreds of crates compile once and cache; only a `Cargo.toml`/`Cargo.lock` change re-runs them |
| Rust version pinned via `ARG` | An unpinned base means a toolchain bump breaks the build with no commit to blame |
| `{{project-name}}-api --health-check` for `HEALTHCHECK` | Distroless has no `curl`. The flag probes `/health` over a raw socket, so the image needs no HTTP client and the binary no TLS stack |
| `ENTRYPOINT` in exec form | The binary is PID 1 and receives SIGTERM directly. Shell form swallows it and graceful shutdown never runs |

Both binaries ship: `{{project-name}}-api` and `{{project-name}}-migrate`.

### Migrations on deploy

Render's free tier doesn't support a pre-deploy hook on any deploy method — see [docs/deployment.md § Migrations on the free tier](deployment.md) for the manual workflow this needs until that changes.

### CI

[`ci.yml`](../.github/workflows/ci.yml) runs on every push to `main`/`stage`/`dev` (and PRs into any of them — a PR into a branch this doesn't list gets no check of its own):

| Job | What it guards |
|---|---|
| `check` | `cargo fmt --check`, `clippy -D warnings`, unit tests (no database) |
| `no-io-in-core` | Fails if `sea-orm`, `sqlx`, `tokio`, `axum`, `hyper`, or `reqwest` reaches the core crate |
| `test` | Full suite against Postgres 18, and **migrations down then up**, so a rollback is known to work before it is needed |
| `audit` | `cargo audit --deny warnings` |
| `docker` | Builds the image, starts it, and asserts `/health`/`/health/ready` — add your own domain smoke assertions here once you have real endpoints |

Deploys happen separately, directly from Render's own Git-connected build — merging into `stage`/`main` triggers it, no publish step in this repo. See [docs/deployment.md](deployment.md).
