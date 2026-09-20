# Contributing

## Project layout

Cargo + bun/turbo workspace:

- `packages/fast-static-server` - routing/handler library
- `packages/fss-cli` - CLI binary (crate `fss-cli`, produces the `fast-static-server` executable)
- `bench/` - throughput/latency benchmark vs npm `serve`

## Setup

- Rust: stable toolchain, edition 2024 (`rust-version = "1.85"` in the workspace `Cargo.toml`)
- [bun](https://bun.sh) for the JS workspace (`bench`, lint/format tooling)

```sh
bun install
```

## Common commands

| Task | Command |
| --- | --- |
| Build | `cargo build --workspace` |
| Test | `cargo test --workspace` |
| Lint (Rust) | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format (Rust) | `cargo fmt --all` |
| Lint (JS) | `bun run lint` (or `turbo run lint`) |
| Format (JS) | `bun run format` / `bun run format:check` |
| Benchmark | `bun run bench` |

All of these run in CI (`.github/workflows/ci.yml`) - a PR won't merge unless they pass.

## Making changes

- Keep the library (`fast-static-server`) and CLI (`fss-cli`) crates separated: the library should stay usable without `clap`/CLI concerns.
- `serve_compat.rs` tests pin behavior to match npm `serve` (status codes, headers, directory listings). If you change routing behavior, update or add a test there rather than only testing internals.
- Dependency versions and `edition`/`license`/`rust-version` live in the root `Cargo.toml` under `[workspace.package]` / `[workspace.dependencies]` - add new crates there and reference with `.workspace = true` in the member `Cargo.toml`, don't duplicate version strings.
- Run `bun run bench` if a change could affect request-handling performance (filesystem access pattern, middleware, dependency swaps). See `bench/README.md` for how to read the results.

## Commits and PRs

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, `refactor:`, ...) - see `git log` for examples.
- Keep PRs focused; a bug fix doesn't need to bundle unrelated refactors.
- Explain *why* in the PR description when a change isn't self-evident from the diff (e.g. a platform-specific bugfix, a benchmark result driving a design choice).

## License

By contributing, you agree your contribution is licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at the project's option (matching the crates' dual-license setup).
