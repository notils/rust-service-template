// cargo-generate does not remove its own config/hook files from the
// generated project on its own — a generated service has no use for either.
file::delete("cargo-generate.toml");
file::delete("hooks");
