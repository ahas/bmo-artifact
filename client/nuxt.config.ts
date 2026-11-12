import { fileURLToPath } from "node:url";

// The document screens as a console bundle: a layer on bmo-ui with its pages, its components, its
// stylesheet and the modules they stand on, attached by its declaration, `console-bundle.json`
// (bmo-ui's `docs/console-bundles.md`; `docs/layout.md`, "The console bundle").
export default defineNuxtConfig({
  extends: ["bmo-ui"],
  // The layer's stylesheet, named from the layer's own folder: an alias in a layer's configuration
  // is read against the host that extends it.
  css: [
    fileURLToPath(new URL("./app/assets/css/document.css", import.meta.url)),
    fileURLToPath(new URL("./app/assets/css/finding.css", import.meta.url))
  ],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
