# Releasing

1. Set `CREATE_IO_API_KEY` in the GitHub repository's Actions secrets to a crates.io API token with `publish-new` permission for the first release. Afterward, replace it with a token limited to `matlas` updates. The secret must exist before publishing the GitHub release.
2. Update the version in `Cargo.toml` and `Cargo.lock`.
3. With MATLAB configured, run the Rust checks and `tests/matlab/run_tests.m` documented in [VALIDATION.md](VALIDATION.md).
4. Run `cargo package --locked` locally and inspect `cargo package --list`.
5. Commit the version, create tag `v<version>`, and publish a GitHub release for that tag.

Publishing the GitHub release runs `.github/workflows/publish.yml`. The workflow rejects a tag that does not match `Cargo.toml`, then publishes with `CARGO_REGISTRY_TOKEN` sourced from `CREATE_IO_API_KEY`.

The workflow uses `--no-verify` because GitHub-hosted runners do not contain the proprietary MATLAB SDK. Local package verification is therefore required before creating the release.
