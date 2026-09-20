// Thin JSR wrapper around the `fss-cli` npm package (prebuilt Rust binary,
// resolved per-platform via npm optionalDependencies).
//
// Run with: deno run -A jsr:@ro80t/fss-cli [args]
import { run } from "npm:fss-cli@0.1.0";

export { run };

if (import.meta.main) {
  Deno.exit(run(Deno.args));
}
