# GitHub Workflows for cauld-ron

This repository includes the following GitHub Actions workflows.

## Workflow overview

### 1. `ci.yml`

- Triggered on pushes to `main` and pull requests targeting `main`
- Runs:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `bash scripts/tokei_check.sh`
  - `cargo build --workspace --all-targets`
  - `cargo nextest run --workspace`
  - `cargo-deny`
  - `cargo-audit`

### 2. `deploy-docs.yml`

- Reserved for a future `doc/` site
- Only triggers when files under `doc/` change

### 3. `release-plz.yml`

- Automates release PR creation on `main`
- Can publish the crate when release metadata is ready

### 4. `release.yml`

- Builds release binaries for Linux, Windows, and macOS when version tags are pushed
- Uploads archived CLI artifacts to the GitHub release

### 5. `update-deps.yml`

- Runs weekly or manually
- Creates a PR if `cargo update` changes `Cargo.lock`

## Supporting repository checks

This repository also carries:

- `scripts/tokei_check.sh` for repository-local structure and lint policy checks
- `clippy.toml` for shared Clippy thresholds
- `deny.toml` for `cargo-deny`
- `.cargo/audit.toml` for `cargo-audit`
- `release-plz.toml` for release-plz behavior
