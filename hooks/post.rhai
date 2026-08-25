// `.github/workflows/*.yml` is excluded from Liquid substitution
// (cargo-generate.toml) because its own `${{ }}` syntax collides with
// Liquid's. This does the small number of real substitutions those files
// need via plain string replacement instead, using non-Liquid tokens
// (`__PROJECT_NAME__` / `__CRATE_NAME__`) that appear literally in the
// template's ci.yml so there's nothing for the collision to corrupt.

let project_name = variable::get("project-name");
let crate_name = variable::get("crate_name");

let ci_path = ".github/workflows/ci.yml";
let ci = file::read(ci_path);
ci = ci.replace("__PROJECT_NAME__", project_name);
ci = ci.replace("__CRATE_NAME__", crate_name);
file::write(ci_path, ci);
