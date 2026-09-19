#!/usr/bin/env node
// Benchmark: fast-static-server (Rust) vs npm `serve`, same fixture, same load profile.
//
// Usage:
//   npm install   (once, installs autocannon + serve as devDependencies)
//   npm run bench [-- --duration=10 --connections=50]

"use strict";

const path = require("node:path");
const fs = require("node:fs");
const os = require("node:os");
const { execFileSync, spawn } = require("node:child_process");
const autocannon = require("autocannon");

const REPO_ROOT = path.resolve(__dirname, "..");
const RUST_BIN = path.join(
  REPO_ROOT,
  "target",
  "release",
  process.platform === "win32" ? "fast-static-server.exe" : "fast-static-server",
);

const args = Object.fromEntries(
  process.argv.slice(2).map((arg) => {
    const [key, value] = arg.replace(/^--/, "").split("=");
    return [key, value ?? true];
  }),
);
const DURATION = Number(args.duration ?? 10);
const CONNECTIONS = Number(args.connections ?? 50);
const LARGE_FILE_MB = 5;

let nextPort = 4600;
function claimPort() {
  return nextPort++;
}

function resolveServeEntry() {
  const pkgJsonPath = require.resolve("serve/package.json");
  const pkg = require(pkgJsonPath);
  const bin = typeof pkg.bin === "string" ? pkg.bin : pkg.bin.serve;
  return path.join(path.dirname(pkgJsonPath), bin);
}

function makeFixture() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "fss-bench-"));
  fs.writeFileSync(
    path.join(dir, "index.html"),
    `<!doctype html><title>bench</title><h1>${"hello world ".repeat(50)}</h1>`,
  );
  fs.writeFileSync(path.join(dir, "large.bin"), Buffer.alloc(LARGE_FILE_MB * 1024 * 1024, 0x61));
  return dir;
}

async function waitForReady(url, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url);
      if (res.ok || res.status === 404) return;
    } catch {
      // not up yet
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`server at ${url} did not become ready in ${timeoutMs}ms`);
}

function killChild(child) {
  return new Promise((resolve) => {
    if (child.exitCode !== null) return resolve();
    child.once("exit", resolve);
    child.kill();
    setTimeout(() => {
      if (child.exitCode === null) child.kill("SIGKILL");
    }, 2000);
  });
}

const SCENARIOS = [
  { name: "small (index.html)", path: "/" },
  { name: `large (${LARGE_FILE_MB}MB file)`, path: "/large.bin" },
];

async function benchmarkTarget(name, spawnServer, fixtureDir) {
  const port = claimPort();
  const child = spawnServer(fixtureDir, port);
  child.stdout?.resume(); // drain, don't inherit: keep console output to our own report
  child.stderr?.resume();

  const baseUrl = `http://127.0.0.1:${port}`;
  await waitForReady(baseUrl + "/");

  const results = {};
  for (const scenario of SCENARIOS) {
    process.stdout.write(`  [${name}] ${scenario.name} ...\n`);
    results[scenario.name] = await autocannon({
      url: baseUrl + scenario.path,
      connections: CONNECTIONS,
      duration: DURATION,
    });
  }

  await killChild(child);
  return results;
}

function toRows(target, results) {
  return SCENARIOS.map((scenario) => {
    const r = results[scenario.name];
    return {
      target,
      scenario: scenario.name,
      "req/sec": Math.round(r.requests.average),
      "latency avg (ms)": r.latency.average,
      "latency p99 (ms)": r.latency.p99,
      "throughput (MB/s)": (r.throughput.average / 1024 / 1024).toFixed(2),
      errors: r.errors,
    };
  });
}

async function main() {
  console.log(`Building release binary (cargo build --release) ...`);
  execFileSync("cargo", ["build", "--release"], { cwd: REPO_ROOT, stdio: "inherit" });
  if (!fs.existsSync(RUST_BIN)) {
    throw new Error(`expected release binary at ${RUST_BIN}, but it wasn't produced`);
  }

  const serveEntry = resolveServeEntry();
  const fixtureDir = makeFixture();
  console.log(`Fixture: ${fixtureDir}`);
  console.log(`Load profile: ${CONNECTIONS} connections, ${DURATION}s per scenario\n`);

  const rustResults = await benchmarkTarget(
    "fast-static-server",
    (dir, port) => spawn(RUST_BIN, [dir, "-l", String(port)]),
    fixtureDir,
  );

  const serveResults = await benchmarkTarget(
    "serve",
    (dir, port) => spawn(process.execPath, [serveEntry, dir, "-l", String(port)]),
    fixtureDir,
  );

  fs.rmSync(fixtureDir, { recursive: true, force: true });

  console.log("\nResults:");
  console.table([...toRows("fast-static-server", rustResults), ...toRows("serve", serveResults)]);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
