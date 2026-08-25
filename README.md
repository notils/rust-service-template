# {{project-name}}

> Generated from [notils/rust-service-template](https://github.com/notils/rust-service-template) — a Rust/axum/Postgres/Render service skeleton, extracted from `notils-praman`.

## Documentation

| Doc | What it answers |
|-----|-----------------|
| [docs/architecture.md](docs/architecture.md) | Design decisions — fill this in as you make them |
| [docs/development.md](docs/development.md) | Local setup, migrations, testing |
| [docs/deployment.md](docs/deployment.md) | Render runbook |

## Quick start

```bash
cp .env.example .env
docker compose up -d postgres
cargo run -p {{project-name}}-migration -- up
cargo run -p {{project-name}}-api
```

```bash
curl localhost:8080/health
curl localhost:8080/health/ready
```

Full setup: [docs/development.md](docs/development.md).

## What's here, what isn't

This is a working starting point, not a finished service:

- `crates/{{project-name}}-types`, `-core`, `-db`, `-migration`, `-api` are real, buildable, minimal skeletons — replace `{{project-name}}-core`'s `placeholder()` and `{{project-name}}-migration`'s example migration with your actual domain logic and schema.
- The Dockerfile, CI (`ci.yml`/`release.yml`), and `render.yaml` are ready to use as-is — only names need to already be right (cargo-generate did that).
- `docs/architecture.md` is a skeleton with prompts, not filled-in content — write it as you make real decisions, the way `notils-praman`'s own copy shows.
