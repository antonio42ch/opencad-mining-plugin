# Releasing (maintainers)

What to do to publish a version so that users can install it from the repository link
(see [INSTALL.md](INSTALL.md)).

1. **Version.** `version` in `Cargo.toml` and `plugin.toml` (a test keeps the manifest in sync), the entry in
   [CHANGELOG.md](../CHANGELOG.md) dated, `Cargo.lock` updated (`cargo build`).
2. **Green CI** on the branch to release: `lint`, `test` (Linux, macOS, Windows), `api-next` and `host-smoke`
   (the plugin loaded in the official Open CAD Studio build, with the documented vein example).
3. **Docs up to date:** `OPENMINING_BLESS=1 cargo test -p openmining_ops` regenerates `docs/MCP.md`;
   the examples in `examples/` are checked by tests, so a stale number fails CI.
4. **Merge** the release branch into `main`. When the work is a stack of pull requests, retarget the last one to
   `main` and merge it with a merge commit: GitHub marks the earlier ones as merged because their commits arrive.
5. **Tag** `vX.Y.Z` on that commit and push the tag. The `release` workflow checks that the tag equals the
   `plugin.toml` version, builds Linux x86_64 / Windows x86_64 / macOS aarch64 and attaches
   `opencad.mining-<os>-<arch>.<so|dll|dylib>` plus `plugin.toml` to the GitHub Release.
6. **Check the release page:** the three binaries and `plugin.toml` are there; edit the generated notes if wanted.
7. **Install test from the link**, on a clean Open CAD Studio: *Plugin Manager → Add repository →
   `https://github.com/antonio42ch/opencad-mining-plugin` → pick the new release → Install → restart*; then
   `MIN_STATUS`, and the steps of [examples/veta](../examples/veta/README.md).
8. **Share** the repository page (README) with the users.

`docs/registry-entry.json` is the entry for the Open CAD Studio plugin registry, if the plugin is listed there.
