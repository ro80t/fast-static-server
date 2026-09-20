#!/usr/bin/env node
"use strict";

const { run } = require("./lib.js");

try {
  process.exit(run());
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
