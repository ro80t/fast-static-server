# bench

Throughput/latency comparison: `fast-static-server` (Rust) vs npm `serve`, same fixture, same load.

```sh
bun install                                     # from the repo root (this is a bun workspace)
bun run bench                                   # defaults: 50 connections, 10s per scenario
bun run bench -- --duration=5 --connections=20  # override
```

Builds the release binary, serves a small HTML file and a 5MB file from both servers in turn
(no shared process, sequential runs), drives each with [autocannon](https://github.com/mcollina/autocannon),
and prints a comparison table. Both servers run with default (quiet) settings — `serve`'s CLI
via its Node entry point directly, no `npx` wrapper.
