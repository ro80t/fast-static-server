#!/usr/bin/env bun
// Publishes fss-cli to npm: the per-platform binary packages first, then the main
// wrapper package. Platforms without a built binary in platforms/*/bin/ are skipped
// with a warning rather than blocking the release - npm's optionalDependencies
// already degrade gracefully on install for platforms that aren't published yet.

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import path from "node:path";

const NPM_DIR = import.meta.dir;
const PLATFORMS_DIR = path.join(NPM_DIR, "platforms");

function npmPublish(dir: string): void {
  console.log(`Publishing ${path.relative(NPM_DIR, dir) || "."} ...`);
  execFileSync("npm", ["publish", "--access", "public"], { cwd: dir, stdio: "inherit" });
}

const platforms = readdirSync(PLATFORMS_DIR, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

let publishedAny = false;
for (const platform of platforms) {
  const dir = path.join(PLATFORMS_DIR, platform);
  const binDir = path.join(dir, "bin");
  if (!existsSync(binDir) || readdirSync(binDir).length === 0) {
    console.warn(`skipping ${platform}: no binary in ${path.relative(NPM_DIR, binDir)}`);
    continue;
  }
  npmPublish(dir);
  publishedAny = true;
}

if (!publishedAny) {
  throw new Error(
    "no platform binaries found - build at least one release binary before publishing",
  );
}

npmPublish(NPM_DIR);
