# fast-static-server

A Rust port of the npm [`serve`](https://www.npmjs.com/package/serve) package: serve a directory over HTTP. Built on [axum](https://github.com/tokio-rs/axum) and [tower-http](https://github.com/tower-rs/tower-http).

## Install

```sh
cargo install fss-cli
```

This installs a `fast-static-server` binary.

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

## Crates

This repository is a Cargo workspace:

- [`packages/fast-static-server`](packages/fast-static-server) - the routing/handler library
- [`packages/fss-cli`](packages/fss-cli) - the CLI binary (crate `fss-cli`, produces the `fast-static-server` executable)

## Benchmarks

See [`bench/`](bench) for a throughput/latency comparison against npm `serve` (`bun run bench`).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
