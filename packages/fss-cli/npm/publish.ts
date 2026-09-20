#!/usr/bin/env bun
// Publishes fss-cli to npm: the per-platform binary packages first, then the main
// wrapper package. Each packages/fss-cli/npm/platforms/*/bin/ must already contain
// a built binary (produced by a release build - not part of this script) before
// publishing, otherwise the platform package would ship without one.

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

for (const platform of platforms) {
  const dir = path.join(PLATFORMS_DIR, platform);
  const binDir = path.join(dir, "bin");
  if (!existsSync(binDir) || readdirSync(binDir).length === 0) {
    throw new Error(
      `${platform}: missing ${path.relative(NPM_DIR, binDir)} - build the release binary for this platform before publishing`,
    );
  }
  npmPublish(dir);
}

npmPublish(NPM_DIR);
