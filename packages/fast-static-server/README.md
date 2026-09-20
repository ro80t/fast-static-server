# fast-static-server

Routing/handler library for `fast-static-server`: an [axum](https://github.com/tokio-rs/axum)-based static file server, a Rust port of npm [`serve`](https://www.npmjs.com/package/serve)'s behavior (status codes, headers, directory listings).

For the CLI, see [`fss-cli`](https://crates.io/crates/fss-cli) (or `cargo install fss-cli`).

## Usage

```rust
use fast_static_server::build_app;

let app = build_app(
    std::env::current_dir().unwrap(), // root directory to serve
    false,                            // spa: rewrite unknown routes to index.html
    false,                            // cors: send permissive CORS headers
    false,                            // debug: log every request
    None,                             // cache_control seconds, e.g. Some(3600)
);

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app).await?;
```

`build_app` returns a plain `axum::Router`, so it composes with any other axum middleware/routes.

## Features

- Directory listings (when no `index.html` is present)
- Byte-range requests (`206 Partial Content`)
- Conditional requests (`304 Not Modified` via `Last-Modified`/`If-Modified-Since`)
- SPA fallback mode
- Optional CORS and `Cache-Control` headers
- Path-traversal protection

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/ro80t/fast-static-server/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/ro80t/fast-static-server/blob/main/LICENSE-MIT) at your option.
