import { consoleManifest } from "bmo-ui/shared/console-manifest";

// The service's manifest read through the contract's own reading (bmo-ui's `docs/console-bundles.md`,
// "What a service owes"): the manifest on standard input, the manifest read, its defaults filled, on
// standard output, or the reading's fault on standard error and an exit of 1. The service's Rust test
// hands it the manifest the route answers.

try {
  console.log(JSON.stringify(consoleManifest(JSON.parse(await Bun.stdin.text()) as unknown)));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
