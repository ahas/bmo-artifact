import "./bundle.css";

import DocumentScreen from "../app/pages/artifact/[id].vue";
import ListScreen from "../app/pages/artifact/index.vue";
import ShareScreen from "../app/pages/artifact/share/[token].vue";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";

// The browser module a console loads the document screens from (bmo-ui's `docs/console-bundles.md`,
// "What a bundle is"; `docs/layout.md`, "The console bundle"): the routes of the screens, each under
// the screen path `/artifact`. A shared document's screen stands alone, with no frame of the console's
// around it.
const bundle: ConsoleBundleModule = {
  routes: [
    // The list and one document stand under the console's title bar (bmo-ui's `docs/console-bundles.md`, the
    // title bar).
    { path: "/artifact", name: "artifact", component: ListScreen, meta: { consoleTitleBar: true } },
    { path: "/artifact/:id", name: "artifact-id", component: DocumentScreen, meta: { consoleTitleBar: true } },
    { path: "/artifact/share/:token", name: "artifact-share-token", component: ShareScreen, meta: { layout: false } }
  ]
};

export default bundle;
