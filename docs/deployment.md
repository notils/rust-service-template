# Deployment

| | |
|---|---|
| **Status** | Active |
| **Platform** | Render, connected directly to this GitHub repo — no Blueprint, no separate image registry (ported from `notils-praman`'s own deployment setup — swap if you deploy elsewhere) |
| **Build** | Render builds [`Dockerfile`](../Dockerfile) itself on every push to the tracked branch |
| **Config** | Set in the Render dashboard (service settings + environment variables) — not version-controlled |

This is the step-by-step runbook.

⚠️ **If you generated this project before this doc was rewritten**, you may have a `render.yaml` and a `release.yml` workflow publishing to GHCR. Both are gone from the template — `notils-praman` and `rentdera-api` independently arrived at the same conclusion and dropped both: for a one-or-two-service setup, Render's own Git-connected build does the same job with less to maintain. Delete both files if you still have them; `ci.yml` (format/lint/test) is untouched either way.

---

## 0. Platform choice

**Provisional, cost-conscious starting point** — the same one `notils-praman` used. Both services should run on `plan: free` to start: free instances spin down after ~15 minutes idle (a cold-start delay on the first request after), which is fine for validating the pipeline and unacceptable once real users depend on it. `plan: free` is also what avoids Render's payment-info prompt when creating a service.

Revisit the plan (and Render itself, vs. self-hosting) once real traffic depends on this service, with real cost/traffic numbers instead of guesses.

---

## 1. Environments

Two separate Render Web Services, each its own connected branch, its own env vars, its own database — never shared.

| | Domain | Deploys from | Status |
|---|---|---|---|
| Staging | `REPLACE_WITH_STAGING_DOMAIN` | pushes to `stage` | **Active** |
| Production | `REPLACE_WITH_PRODUCTION_DOMAIN` | pushes to `main` | Create when there's a real reason to — see below |

`dev` is the integration branch everything lands on first — it deploys nowhere on its own. The flow is `dev` → PR into `stage` (deploys staging) → PR into `main` (deploys production), never a direct push from `dev` to either deploy branch. `ci.yml` runs on all three (`main`/`stage`/`dev`) for exactly this reason — a PR into a branch `ci.yml` doesn't list gets no check of its own, only whatever it inherited from its head branch's last push.

🔴 **Once production exists, each environment must have its own signing key(s) and its own `DATABASE_URL`.** A staging-issued credential that verifies in production is a complete authentication bypass — never copy secrets across environments.

---

## 2. First-time setup (per environment)

1. Render Dashboard → **New → Web Service** → connect this GitHub repo (its GitHub App handles private-repo authorization, no PAT needed). Pick the branch to track (`stage` for staging, `main` for production — a separate service each). Render detects `Dockerfile` automatically.
2. **Region:** pick whatever's closest to your users, matching wherever this service's own database ends up.
3. **Health check path:** `/health` (liveness only — never `/health/ready`; a readiness probe here would let a brief database blip restart otherwise-healthy instances).
4. **Environment variables** (dashboard → this service → Environment — never in git): `DATABASE_URL`, `PORT`, `HOST=0.0.0.0`, `LOG_FORMAT=json`, `RUST_LOG`, plus whatever this service's own `config.rs` requires. Any signing key or secret goes here too, per-environment, never shared.
5. **Apply migrations manually first** (§3) — the database has no schema yet. Do this before the first deploy, or the app crash-loops trying to connect to an unmigrated database.
6. Save — Render builds and deploys automatically. Verify (§5).

---

## 3. Migrations on the free tier

Migrations do **not** run automatically on deploy — Render's `plan: free` doesn't support a pre-deploy hook on *any* deploy method, and migrations must never run at application startup either (every instance would race the others). Run them manually, from your machine, pointed at the target database:

```bash
DATABASE_URL="<the same connection string set in Render>" \
  cargo run -p {{project-name}}-migration -- up
```

- Run this against **staging's** database before deploying to staging, and against **production's** before deploying to production.
- Run it *before* the push/merge that needs the new schema lands, not after — a deploy that starts serving against a stale schema is exactly the race a pre-deploy hook exists to prevent.
- `cargo run -p {{project-name}}-migration -- status` shows what's pending against a given `DATABASE_URL` without applying anything.

**Once this moves off `plan: free`:** Render's paid tiers support a pre-deploy command directly in the dashboard. Point it at `{{project-name}}-migrate up` and this step goes away.

---

## 4. Routine deploys

Merge into the tracked branch (`stage` or `main`) — in practice always via a PR, never a direct push, so `ci.yml` runs against the actual merge before it lands. Render builds `Dockerfile` off that branch and deploys automatically — no separate deploy build, no manual "deploy latest commit" click, no drift between what's merged and what's running.

⚠️ **If your change includes a migration, run §3 *before* the merge lands** — getting the order backwards (deploy before migrate) fails the same way either direction: running code expecting columns the database doesn't have yet, or the reverse.

The flow is always `dev` → `stage` → `main`, never `dev` straight to `main` — staging is what a `main`-bound PR gets validated against before production sees it.

---

## 5. Verifying a deploy

```bash
curl -sf https://REPLACE_WITH_STAGING_DOMAIN/health          # liveness — process only
curl -sf https://REPLACE_WITH_STAGING_DOMAIN/health/ready     # readiness — includes a DB round-trip
```

If `/health/ready` returns `503`: on `plan: free` this usually means you skipped §3 and the schema doesn't exist yet.

---

## 6. Rolling back

Render's Git-connected deploys keep a deploy history per service — dashboard → the service → **Events**/**Deploys** → pick a prior successful deploy → **Rollback to this deploy**. No digest pinning needed, unlike the old image-based setup: Render already has every previous build.

Fix forward on `dev`/`stage`/`main` as usual; the rollback just buys time while that happens.

---

## 7. Known advisories

`.cargo/audit.toml` ships empty — nothing has needed an ignore yet. When one does, document it the way `notils-praman` does: reason the vulnerable path is unreachable, plus a removal condition, per advisory ID.
