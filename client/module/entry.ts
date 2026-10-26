import "./bundle.css";

import DocumentScreen from "../app/pages/archive/[id].vue";
import ListScreen from "../app/pages/archive/index.vue";
import ShareScreen from "../app/pages/archive/share/[token].vue";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";

// The browser module a console loads the document screens from (bmo-ui's `docs/console-bundles.md`,
// "What a bundle is"; `docs/layout.md`, "The console bundle"): the routes of the screens, each under
// the screen path `/archive`. A shared document's screen stands alone, with no frame of the console's
// around it.
const bundle: ConsoleBundleModule = {
  routes: [
    // The list and one document stand under the console's title bar (bmo-ui's `docs/console-bundles.md`, the
    // title bar).
    { path: "/archive", name: "archive", component: ListScreen, meta: { consoleTitleBar: true } },
    { path: "/archive/:id", name: "archive-id", component: DocumentScreen, meta: { consoleTitleBar: true } },
    { path: "/archive/share/:token", name: "archive-share-token", component: ShareScreen, meta: { layout: false } }
  ]
};

export default bundle;
