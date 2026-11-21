import { fileURLToPath } from "node:url";

// The document screens stood alone for the browser tests: the client's layer, which carries bmo-ui's, under a page
// shell of its own in place of a console's. What the host builds and caches stands under the client's packages,
// where it resolves them and no checkout file sees it.
const cache = fileURLToPath(new URL("../../../node_modules/.cache/artifact-browser-host", import.meta.url));

export default defineNuxtConfig({
  extends: [fileURLToPath(new URL("../../..", import.meta.url))],
  // A console draws a bundle's screens in the browser alone, and the screens' calls go to the stand-in the test
  // puts in front of the page.
  ssr: false,
  buildDir: `${cache}/nuxt`,
  vite: {
    cacheDir: `${cache}/vite`,
    // The CommonJS packages the layer's modules import, bundled before the first page so the browser can read them;
    // the layer's own modules lean on what Nuxt imports for them, so they are never pre-bundled apart from the app.
    optimizeDeps: { include: ["dayjs", "dayjs/plugin/utc", "dayjs/plugin/timezone"], exclude: ["bmo-ui"] }
  },
  devtools: { enabled: false },
  telemetry: false,
  // The icons the screens name, and the one the layer names in its settings that no scan reads, bundled into the
  // page, so a test asks the network for none.
  icon: { clientBundle: { scan: true, icons: ["material-symbols:menu"] }, fallbackToApi: false }
});
