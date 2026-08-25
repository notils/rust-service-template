# Deployment

| | |
|---|---|
| **Status** | Active |
| **Platform** | Render (ported from notils-praman's own deployment setup — swap if you deploy elsewhere) |
| **Config** | [`render.yaml`](../render.yaml) |
| **Image** | `ghcr.io/REPLACE_WITH_ORG/{{project-name}}` |

This is the step-by-step runbook.

---

## 0. Platform choice

`render.yaml` ships with `plan: free` on both services and production commented out — the same provisional, cost-conscious starting point notils-praman used. Revisit both (the plan tier, and Render itself) once real traffic depends on this service, with real cost/traffic numbers rather than guesses.

`plan: free` also avoids Render's payment-info requirement for Blueprint services (only paid instance types trigger it).

---

## 1. Environments

| | Domain | Image tag | Status |
|---|---|---|---|
| Staging | `REPLACE_WITH_STAGING_DOMAIN` | `:dev` | Active |
| Production | `REPLACE_WITH_PRODUCTION_DOMAIN` | `:latest` | Commented out in `render.yaml` until there's a `main` branch and a real reason to deploy it |

### 1a. Pulling a private GHCR image

If your repo is private, its GHCR package is private too, and Render needs a credential to pull it:

1. Generate a GitHub PAT (**classic**, not fine-grained — GHCR container-package reads are unreliable with fine-grained tokens even when scopes look right) with the `read:packages` scope.
2. Render Dashboard → **Workspace Settings → Credentials** → add a registry credential: registry `ghcr.io`, your GitHub username, the PAT as the password, and a **Name** you choose.
3. `image.creds` in `render.yaml` is an object, not a plain string:
   ```yaml
   image:
     creds:
       fromRegistryCreds:
         name: the-name-you-chose-in-step-2
   ```
   Both `image.creds` entries reference this — update both if you ever recreate the credential under a different name.

⚠️ Do this **before** applying the Blueprint, or the service fails to pull its image with a misleading "not found"/"could not be fetched" (Render can't distinguish a missing image from one it isn't authorized to see).

---

## 2. First-time setup

1. Create the registry credential (§1a) if the repo is private.
2. Render Dashboard → **New → Blueprint** → point at this repo → it reads [`render.yaml`](../render.yaml) and creates the staging service.
3. Fill in `DATABASE_URL` for the staging service (`sync: false` in the blueprint, so it's never in git) — a hosted Postgres (Neon, Render's own managed Postgres, etc.), not your local `docker compose` instance, which Render's containers cannot reach.
4. **Apply migrations manually first** (§3) — `plan: free` doesn't run `preDeployCommand`, so the database has no schema yet. Do this before the first deploy, or the app crash-loops trying to connect to an unmigrated database.
5. Deploy, then verify (§5).

---

## 3. Migrations on the free tier

🔴 **`render.yaml` deliberately has no `preDeployCommand`** — Render does not support it on `plan: free`, only on paid tiers. Migrations do **not** run automatically on deploy, and must never run at application startup either (every instance would race the others). Run them manually, from your machine, pointed at the target database:

```bash
DATABASE_URL="<the same connection string set in Render>" \
  cargo run -p {{project-name}}-migration -- up
```

- Run this against **staging's** database before deploying to staging, and against **production's** before deploying to production.
- Run it *before* the deploy that needs the new schema, not after.
- `cargo run -p {{project-name}}-migration -- status` shows what's pending without applying anything.

**Once this moves off `plan: free`:** add `preDeployCommand: /usr/local/bin/{{project-name}}-migrate up` to the service(s) in `render.yaml` and this step goes away.

---

## 4. Routine deploys

🔴 **Merging to `dev` builds a new image — it does not deploy it.** Render's auto-deploy only applies to services built from a git repo; an image-based (`runtime: image`) service like this one is [not automatically redeployed when its floating tag gets a new digest](https://render.com/docs/deploys) — you have to trigger it.

```
push to dev ──► CI ──► release.yml builds & pushes ghcr.io/.../{{project-name}}:dev ──► (new digest sits in GHCR, unused)
                                                                                             │
                                                                 a manual step is required ──┘
```

**After merging, manually deploy:** Render dashboard → the staging service → **Manual Deploy → Deploy latest commit** (or a deploy hook called from `release.yml`, which is worth wiring up once this bites someone once).

⚠️ **On `plan: free`, deploying also doesn't apply migrations** — run §3 *before* triggering the deploy if your change includes one.

---

## 5. Verifying a deploy

```bash
curl -sf https://REPLACE_WITH_STAGING_DOMAIN/health          # liveness — process only
curl -sf https://REPLACE_WITH_STAGING_DOMAIN/health/ready     # readiness — includes a DB round-trip
```

If `/health/ready` returns `503`: on `plan: free`, this usually means you skipped §3 and the schema doesn't exist yet.

---

## 6. Rolling back

`render.yaml` pins each service to a floating tag (`:latest` / `:dev`), so a rollback means pointing the service at a specific `:sha-<digest>` tag instead:

1. Find the digest of the last known-good build (GHCR package page, or a prior Actions run's `docker` job output).
2. In the Render dashboard, temporarily override the service's image to `ghcr.io/.../{{project-name}}:sha-<digest>` and deploy.
3. Fix forward on `dev`/`main`; remove the override once the fix is out.

This is why every build gets an immutable `:sha-` tag — a rollback names a digest, not a guess about what `latest` used to be.

---

## 7. Known advisories

`.cargo/audit.toml` ships empty — nothing has needed an ignore yet. When one does, document it the way notils-praman does: reason the vulnerable path is unreachable, plus a removal condition, per advisory ID.
