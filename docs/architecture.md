# Architecture

| | |
|---|---|
| **Status** | Draft — fill this in as real decisions get made |
| **Decided** | REPLACE_WITH_DATE |

This skeleton is ported from notils-praman's own `architecture.md`, which found this shape worked well for keeping a service "minimal now, extensible later" without those two goals fighting each other. Replace each section's placeholder with your own service's actual decisions — don't leave a section saying nothing rather than delete it.

---

## 0. The design constraint

<!--
What tension is this service resolving? notils-praman's was:
"Keep it minimal so it doesn't block anything, but it must fully support
[X] — a fully independent, powerful [Y]."
State yours, then the 2-3 structural decisions below that resolve it.
-->

---

## 1. Crate layout

```
{{project-name}}/
├── crates/
│   ├── {{project-name}}-types/     shared types, error enums. No dependencies
│   ├── {{project-name}}-core/      domain logic. NO I/O
│   ├── {{project-name}}-db/        Postgres (SeaORM), migrations, repositories
│   └── {{project-name}}-api/       HTTP (axum), routes, middleware
├── migrations/
└── docs/
```

🔴 **`{{project-name}}-core` must not depend on `{{project-name}}-db` or any I/O crate.** Domain logic that cannot touch the network or database is testable without fixtures. CI's `no-io-in-core` job enforces this mechanically.

---

## 2. The extension point that matters most

<!--
What's the ONE thing this service needs to be easy to extend without a
rewrite? notils-praman's was "adding a new auth provider" — a trait +
registry, with issuance kept in one place so a new provider can't
accidentally issue something wrong.

Name yours here, sketch the trait/interface, and give a concrete "adding X
later, concretely" walkthrough the way notils-praman does for phone OTP.
-->

---

## 3. Data model

<!--
The load-bearing schema decisions — not every table, just the ones a future
reader needs explained (why this is normalized this way, why a column is
nullable, why something is TEXT instead of an enum). Sketch the real SQL,
with comments explaining *why*, the way `m20260101_000001_create_example_table`
does for its one column.
-->

---

## 4. The stable contract

<!--
What do consumers of this service depend on, that you're committing to never
break without a version bump? At minimum: the error envelope shape (already
fixed by {{project-name}}-types::ErrorEnvelope) and whatever your primary
API surface is.
-->

| Surface | Guarantee |
|---------|-----------|
| Error envelope | `{ error: { code, message, request_id } }` |

**Everything else is free to change** — internal crate structure, storage details, even the language.

---

## 5. Deliberate non-choices

<!--
Things you considered and rejected, with why — this is what stops someone
re-litigating a decision six months from now without knowing it was already
weighed.
-->

| Rejected | Why |
|----------|-----|
| Building an authorization engine | *(if applicable)* — issue claims, let the caller decide access to specific resources |
| Generic plugin system | A trait + registry covers the real cases; a plugin loader is a project, not a feature |

---

## 6. Security baseline

<!--
Whatever's load-bearing for this service specifically — not a generic
checklist. notils-praman's example: Argon2id for passwords, cargo audit in
CI from commit one, vetted crypto crates only, rate limiting by IP AND
identifier, constant-time comparison for tokens/codes, secrets never
committed and separate per environment, failures logged without the
attempted credential.
-->
