import { fileURLToPath } from "node:url";

// The archive screen as a layer: its pages, its components, its stylesheet and the modules they stand
// on, on top of bmo-ui. A host extends it as it extends bmo-ui, and brings its i18n module, its
// language pack and the components this layer leaves to it (`docs/layout.md`, "The console plugin
// bundle").
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
