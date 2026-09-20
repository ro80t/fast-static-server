# fast-static-server

A Rust port of the npm [`serve`](https://www.npmjs.com/package/serve) package: serve a directory over HTTP. Built on [axum](https://github.com/tokio-rs/axum) and [tower-http](https://github.com/tower-rs/tower-http).

## Install

```sh
cargo install fss-cli
```

npm (`npm install -g fss-cli`) and JSR (`deno install -gA jsr:@robot_official/fss-cli`) distributions are set up (see [`packages/fss-cli/npm`](packages/fss-cli/npm) / [`packages/fss-cli/jsr`](packages/fss-cli/jsr)) but not yet published - the release pipeline that builds and ships prebuilt binaries for them doesn't exist yet. `cargo install` is the only working install path today.

## Usage

```sh
fast-static-server [OPTIONS] [DIR]
```

```
Arguments:
  [DIR]  Directory to serve [default: .]

Options:
  -l, --listen <PORT>            Port to listen on [default: 3000]
      --host <HOST>              Host/interface to bind to [default: 0.0.0.0]
  -s, --single                   Single Page App mode: rewrite all not-found requests to index.html
      --cors                     Send permissive CORS headers (Access-Control-Allow-Origin: *)
  -d, --debug                    Show debug info (log every request)
  -c, --cache-control <SECONDS>  Send `Cache-Control: public, max-age=<seconds>` for served files (off by default)
  -h, --help                     Print help
```

## Features

- Directory listings (when no `index.html` is present)
- Byte-range requests (`206 Partial Content`)
- Conditional requests (`304 Not Modified` via `Last-Modified`/`If-Modified-Since`)
- SPA fallback mode (`-s`)
- Optional CORS and `Cache-Control` headers
- Path-traversal protection

## Repository layout

A Cargo workspace (Rust) plus a bun/turbo workspace (JS tooling):

- [`packages/fast-static-server`](packages/fast-static-server) - the routing/handler library
- [`packages/fss-cli`](packages/fss-cli) - the CLI binary (crate `fss-cli`, produces the `fast-static-server` executable)
  - [`npm/`](packages/fss-cli/npm) - npm distribution: a thin JS wrapper that resolves the right prebuilt binary via per-platform `optionalDependencies`
  - [`jsr/`](packages/fss-cli/jsr) - JSR distribution: wraps the npm package via an `npm:` specifier
- [`bench/`](bench) - throughput/latency benchmark vs npm `serve` (TypeScript, run with bun)

## Development

See [`.github/contributing.md`](.github/contributing.md) for setup, common commands (build/test/lint/format/bench), and PR conventions.

## Benchmarks

```sh
bun run bench
```

See [`bench/`](bench) for details on the load profile and how to read the results.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
