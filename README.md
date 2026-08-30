# {{project-name}}

> Generated from [notils/rust-service-template](https://github.com/notils/rust-service-template) — a Rust/axum/Postgres/Render service skeleton, extracted from `notils-praman`.

## Generating a new service from this template

Install `cargo-generate` once (needs a Rust toolchain — `rustup` installs one):

```bash
cargo install cargo-generate
```

Then generate. Pass the **short service name** (matching the `praman`/`rentdera` convention — this becomes the crate prefix: `<name>-types`, `<name>-core`, `<name>-db`, `<name>-migration`, `<name>-api`), not the eventual repo name — `myservice` below is a placeholder, substitute your own:

```bash
cargo generate --git notils/rust-service-template --name myservice
```

`cargo-generate` writes the output into a folder named after `--name` (`myservice/` here). Rename it to match the repo-naming convention before pushing:

```bash
mv myservice myservice-api
cd myservice-api
```

`cargo-generate` already ran `git init` for you (no commits yet). Do the one required manual step (see [Quick start](#quick-start) below for why), then commit and push:

```bash
cargo fmt --all
git add -A && git commit -m "Initial scaffold from notils/rust-service-template"
gh repo create notils/myservice-api --private --source=. --push
```

## Documentation

| Doc | What it answers |
|-----|-----------------|
| [docs/architecture.md](docs/architecture.md) | Design decisions — fill this in as you make them |
| [docs/development.md](docs/development.md) | Local setup, migrations, testing |
| [docs/deployment.md](docs/deployment.md) | Render runbook |

## Quick start

Run this once, right after generating, before your first commit:

```bash
cargo fmt --all
```

🔴 Not optional cleanup — rustfmt's import order and line-wrapping depend on
your project name's exact length and alphabetical position (e.g. whether it
sorts before or after `sea_orm`), so the freshly generated files will not
already match what `cargo fmt --check` demands in CI. This is a one-time fix.

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
- The Dockerfile and CI (`ci.yml`) are ready to use as-is — only names need to already be right (cargo-generate did that). Deployment is a Render Web Service connected directly to this repo (`docs/deployment.md`) — no `render.yaml`, no image registry, nothing else to generate.
- `docs/architecture.md` is a skeleton with prompts, not filled-in content — write it as you make real decisions, the way `notils-praman`'s own copy shows.
