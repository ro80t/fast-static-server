"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");

const PLATFORM_PACKAGES = {
  "linux-x64": "@robot_official/fss-cli-linux-x64",
  "linux-arm64": "@robot_official/fss-cli-linux-arm64",
  "darwin-x64": "@robot_official/fss-cli-darwin-x64",
  "darwin-arm64": "@robot_official/fss-cli-darwin-arm64",
  "win32-x64": "@robot_official/fss-cli-win32-x64",
};

function resolveBinary() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (!pkg) {
    throw new Error(`fss-cli: unsupported platform ${key}`);
  }
  const bin = process.platform === "win32" ? "fast-static-server.exe" : "fast-static-server";
  let binPath;
  try {
    binPath = require.resolve(`${pkg}/bin/${bin}`);
  } catch {
    throw new Error(
      `fss-cli: missing the ${pkg} optional dependency. Reinstall with npm/bun/pnpm so it can be fetched for your platform.`,
    );
  }
  // npm tarballs built on Windows don't reliably preserve the executable bit for
  // Unix binaries - set it defensively rather than fail to spawn on Linux/macOS.
  if (process.platform !== "win32") {
    fs.chmodSync(binPath, 0o755);
  }
  return binPath;
}

function run(args = process.argv.slice(2)) {
  const result = spawnSync(resolveBinary(), args, { stdio: "inherit" });
  return result.status ?? 1;
}

module.exports = { resolveBinary, run };
