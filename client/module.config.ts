import { existsSync, readFileSync, readdirSync } from "node:fs";
import { basename } from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { defineConfig } from "vite";

import type { Plugin } from "vite";

// The document screens built as the browser module a console loads (bmo-ui's `docs/console-bundles.md`,
// "What a bundle is"; `docs/layout.md`, "The console bundle"): one ECMAScript module, its stylesheet and
// its words under `dist/`, which the service answers. Every import of a library the console shares stays an
// import, written as the package's own path, and everything else is carried in the module.

const SHARED = ["vue", "vue-router", "vue-i18n", "nuxt/app", "@nuxt/ui", "bmo-ui"];

const here = (path: string): string => fileURLToPath(new URL(path, import.meta.url));

function shared(id: string): boolean {
  return SHARED.some((name) => id === name || id.startsWith(`${name}/`));
}

// A component a screen names without importing it: the bundle's own from its folder, Nuxt UI's by its
// name without the prefix, and the layer's where the layer has a file by that name.
function componentOf(name: string): { name: string; from: string } | undefined {
  if (name === "NuxtLink") return { name: "default", from: here("./module/nuxt-link.ts") };
  if (/^U[A-Z]/.test(name)) return { name: "default", from: `@nuxt/ui/components/${name.slice(1)}.vue` };
  if (existsSync(here(`./node_modules/bmo-ui/app/components/${name}.vue`))) return { name: "default", from: `bmo-ui/app/components/${name}.vue` };

  return undefined;
}

// The bundle's words as the service answers them: each locale's pack under the locale's code.
function words(): Plugin {
  return {
    name: "artifact-words",
    generateBundle() {
      const folder = here("./i18n");
      const packs = Object.fromEntries(
        readdirSync(folder)
          .filter((file) => file.endsWith(".json"))
          .sort()
          .map((file) => [basename(file, ".json"), JSON.parse(readFileSync(`${folder}/${file}`, "utf8")) as unknown])
      );

      this.emitFile({ type: "asset", fileName: "words.json", source: `${JSON.stringify(packs)}\n` });
    }
  };
}

export default defineConfig({
  // The module runs in the browser alone: what Nuxt states of the side a file runs on is stated here.
  define: { "import.meta.client": "true", "import.meta.server": "false" },
  plugins: [
    vue(),
    tailwindcss(),
    AutoImport({
      imports: [
        "vue",
        {
          "nuxt/app": ["useAsyncData", "useFetch", "useHead", "useNuxtApp", "useRequestURL", "useRoute", "useRouter", "useRuntimeConfig"],
          [here("./module/page-meta.ts")]: ["definePageMeta"]
        }
      ],
      dts: false
    }),
    Components({ dirs: ["app/components"], resolvers: [componentOf], dts: false }),
    words()
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: { entry: here("./module/entry.ts"), formats: ["es"], fileName: () => "bundle.js", cssFileName: "bundle" },
    rolldownOptions: { external: shared, output: { codeSplitting: false } }
  }
});
