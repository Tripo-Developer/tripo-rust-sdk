# Releasing

Releases are managed by [release-plz](https://release-plz.dev/) through
`.github/workflows/publish.yml` and configured in `release-plz.toml`.
Publishing uses crates.io Trusted Publishing: release-plz obtains a temporary
token through GitHub Actions OIDC.

## Cutting a release

1. Merge changes into `main`. The release workflow runs the reusable CI
   workflow, including formatting, clippy, tests, and the Rust 1.88 MSRV build.
2. After CI succeeds, release-plz creates or updates a release PR containing
   the next version in `Cargo.toml` and the release notes in `CHANGELOG.md`.
   CI is explicitly dispatched on the release PR branch using `GITHUB_TOKEN`.
3. Review and merge the release PR. After CI succeeds on `main`, release-plz
   publishes the crate to crates.io, creates the `vX.Y.Z` tag, and creates a
   GitHub Release containing the changelog. docs.rs builds the Rust API
   documentation after the crate is published.

`release_always = false` makes the merged release PR the publishing trigger.
The release workflow can also be rerun manually on `main` through
**Actions → Release → Run workflow**.

The jobs that create release PRs and publish releases run only in
`VAST-AI-Research/tripo-rust-sdk` on `main`.

The `User-Agent` derives from `CARGO_PKG_VERSION`, so it tracks `Cargo.toml`
automatically.

The repository does not track `Cargo.lock`. CI and publishing resolve
dependencies from `Cargo.toml` and generate a local lockfile. Dependency
updates can raise the effective MSRV, so after changing dependencies,
re-check `rust-version` against the `msrv` job in `.github/workflows/ci.yml`.
CI fails if the two disagree.

## GitHub Actions setup

In **Settings → Actions → General → Workflow permissions**, enable
**Allow GitHub Actions to create and approve pull requests**. The workflow
uses the built-in `GITHUB_TOKEN` to create release PRs, dispatch their CI,
push tags, and create GitHub Releases.

## crates.io Trusted Publishing setup

On [crates.io](https://crates.io/crates/tripo3d-sdk/settings) → **Trusted
Publishing** → **Add a new publisher** → **GitHub**:

| Field | Value |
| --- | --- |
| Repository owner | `VAST-AI-Research` |
| Repository name | `tripo-rust-sdk` |
| Workflow filename | `publish.yml` |
| Environment | *(blank)* |

Trusted Publishing cannot bootstrap a brand-new crate — the first version must
be published manually, which for this crate already happened at `0.1.0`.
The workflow remains named `publish.yml` to match this publisher entry.
