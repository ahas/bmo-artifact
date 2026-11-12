import { fileURLToPath } from "node:url";

// The Nuxt app the bundle's sources are prepared and type-checked in: bmo-ui extended with the bundle's pages,
// components and stylesheets, as a console runs them (`docs/layout.md`, "The console bundle"). The browser
// module a console loads is built by `module.config.ts`, not by Nuxt.
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
