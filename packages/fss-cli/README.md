# fss-cli

CLI for `fast-static-server`: serve a directory over HTTP, a fast Rust port of npm [`serve`](https://www.npmjs.com/package/serve).

## Install

```sh
cargo install fss-cli
```

```sh
npm install -g fss-cli
```

```sh
deno install -gA jsr:@robot_official/fss-cli
```

All three install the same `fast-static-server` executable. The npm and JSR installs fetch a prebuilt binary for your platform (linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64) - no Rust toolchain needed.

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

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/ro80t/fast-static-server/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/ro80t/fast-static-server/blob/main/LICENSE-MIT) at your option.
