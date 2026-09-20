#!/usr/bin/env bun
// Benchmark: fast-static-server (Rust) vs npm `serve`, same fixture, same load profile.

import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { execFile, execFileSync, spawn, type ChildProcess } from "node:child_process";
import autocannon from "autocannon";

const REPO_ROOT = path.resolve(import.meta.dirname, "..");
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
function claimPort(): number {
  return nextPort++;
}

function resolveServeEntry(): string {
  const pkgJsonPath = require.resolve("serve/package.json");
  const pkg = require(pkgJsonPath);
  const bin = typeof pkg.bin === "string" ? pkg.bin : pkg.bin.serve;
  return path.join(path.dirname(pkgJsonPath), bin);
}

function makeFixture(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "fss-bench-"));
  fs.writeFileSync(
    path.join(dir, "index.html"),
    `<!doctype html><title>bench</title><h1>${"hello world ".repeat(50)}</h1>`,
  );
  fs.writeFileSync(path.join(dir, "large.bin"), Buffer.alloc(LARGE_FILE_MB * 1024 * 1024, 0x61));
  return dir;
}

async function waitForReady(url: string, timeoutMs = 5000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url);
      if (res.ok || res.status === 404) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`server at ${url} did not become ready in ${timeoutMs}ms`);
}

function killChild(child: ChildProcess): Promise<void> {
  return new Promise((resolve) => {
    if (child.exitCode !== null) return resolve();
    child.once("exit", () => resolve());
    child.kill();
    setTimeout(() => {
      if (child.exitCode === null) child.kill("SIGKILL");
    }, 2000);
  });
}

interface Scenario {
  name: string;
  path: string;
}

const SCENARIOS: Scenario[] = [
  { name: "small (index.html)", path: "/" },
  { name: `large (${LARGE_FILE_MB}MB file)`, path: "/large.bin" },
];

const CURL_DEVNULL = process.platform === "win32" ? "NUL" : "/dev/null";
const CROSS_CHECK_DURATION = 3;

interface CurlResult {
  ms: number;
  bytes: number;
}

function curlOnce(url: string): Promise<CurlResult> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    execFile("curl", ["-s", "-o", CURL_DEVNULL, "-w", "%{size_download}", url], (err, stdout) => {
      if (err) return reject(err);
      resolve({ ms: Date.now() - start, bytes: Number(stdout) });
    });
  });
}

interface CurlCheck {
  "req/sec": number;
  "latency avg (ms)": number;
  "throughput (MB/s)": string;
  errors: number;
}

// Bypasses autocannon (one TCP connection per curl process) to tell a slow "large"
// result apart from the benchmark client itself being the bottleneck.
async function curlCrossCheck(url: string, connections: number): Promise<CurlCheck> {
  const deadline = Date.now() + CROSS_CHECK_DURATION * 1000;
  const start = Date.now();
  let count = 0;
  let totalBytes = 0;
  let totalMs = 0;
  let errors = 0;

  while (Date.now() < deadline) {
    const batch = await Promise.all(
      Array.from({ length: connections }, () =>
        curlOnce(url).catch(() => {
          errors++;
          return null;
        }),
      ),
    );
    for (const r of batch) {
      if (!r) continue;
      count++;
      totalBytes += r.bytes;
      totalMs += r.ms;
    }
  }

  const elapsedSec = (Date.now() - start) / 1000;
  return {
    "req/sec": Math.round(count / elapsedSec),
    "latency avg (ms)": Number((totalMs / count).toFixed(2)),
    "throughput (MB/s)": (totalBytes / elapsedSec / 1024 / 1024).toFixed(2),
    errors,
  };
}

interface AutocannonResult {
  requests: { average: number };
  latency: { average: number; p99: number };
  throughput: { average: number };
  errors: number;
}

interface BenchmarkOutcome {
  results: Record<string, AutocannonResult>;
  curlCheck: CurlCheck;
}

async function benchmarkTarget(
  name: string,
  spawnServer: (fixtureDir: string, port: number) => ChildProcess,
  fixtureDir: string,
): Promise<BenchmarkOutcome> {
  const port = claimPort();
  const child = spawnServer(fixtureDir, port);
  child.stdout?.resume(); // drain without inheriting
  child.stderr?.resume();

  const baseUrl = `http://127.0.0.1:${port}`;
  await waitForReady(baseUrl + "/");

  const results: Record<string, AutocannonResult> = {};
  for (const scenario of SCENARIOS) {
    process.stdout.write(`  [${name}] ${scenario.name} ...\n`);
    results[scenario.name] = await autocannon({
      url: baseUrl + scenario.path,
      connections: CONNECTIONS,
      duration: DURATION,
      // avoids autocannon's single-threaded receive path bottlenecking large-file runs
      workers: 4,
    });
  }

  process.stdout.write(`  [${name}] large file, curl cross-check ...\n`);
  const largeScenario = SCENARIOS.find((s) => s.path === "/large.bin")!;
  const curlCheck = await curlCrossCheck(baseUrl + largeScenario.path, CONNECTIONS);

  await killChild(child);
  return { results, curlCheck };
}

interface BenchRow {
  target: string;
  scenario: string;
  "req/sec": number;
  "latency avg (ms)": number;
  "latency p99 (ms)": number;
  "throughput (MB/s)": string;
  errors: number;
}

function toRows(target: string, results: Record<string, AutocannonResult>): BenchRow[] {
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

async function main(): Promise<void> {
  console.log(`Building release binary (cargo build --release) ...`);
  execFileSync("cargo", ["build", "--release"], { cwd: REPO_ROOT, stdio: "inherit" });
  if (!fs.existsSync(RUST_BIN)) {
    throw new Error(`expected release binary at ${RUST_BIN}, but it wasn't produced`);
  }

  const serveEntry = resolveServeEntry();
  const fixtureDir = makeFixture();
  console.log(`Fixture: ${fixtureDir}`);
  console.log(`Load profile: ${CONNECTIONS} connections, ${DURATION}s per scenario\n`);

  const rust = await benchmarkTarget(
    "fast-static-server",
    (dir, port) => spawn(RUST_BIN, [dir, "-l", String(port)]),
    fixtureDir,
  );

  const serve = await benchmarkTarget(
    "serve",
    (dir, port) => spawn(process.execPath, [serveEntry, dir, "-l", String(port)]),
    fixtureDir,
  );

  fs.rmSync(fixtureDir, { recursive: true, force: true });

  console.log("\nResults:");
  console.table([...toRows("fast-static-server", rust.results), ...toRows("serve", serve.results)]);

  console.log(
    `\nLarge-file curl cross-check (${CONNECTIONS} connections, ${CROSS_CHECK_DURATION}s, one TCP connection per request - no shared client bottleneck):`,
  );
  console.table([
    { target: "fast-static-server", ...rust.curlCheck },
    { target: "serve", ...serve.curlCheck },
  ]);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
