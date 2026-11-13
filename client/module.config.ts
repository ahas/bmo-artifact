import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import AutoImport from "unplugin-auto-import/vite";
import Components from "unplugin-vue-components/vite";
import { consoleBundle } from "bmo-ui/bundle/console-bundle.js";
import { defineConfig } from "vite";

import type { Plugin } from "vite";

// The document screens built as the browser module a console loads, with bmo-ui's bundle build
// (`docs/layout.md`, "The console bundle"): one ECMAScript module, its stylesheet, its words and the layer it
// was built on under `dist/`, which the console face answers, and bmo-ui's audio player with its words under
// `player/`. The bundle build leaves the libraries the console shares unbundled; everything else is carried in
// the module.

const here = (path: string): string => fileURLToPath(new URL(path, import.meta.url));

// A component a screen names without importing it: the bundle's own from its folder, Nuxt UI's by its
// name without the prefix, and the layer's where the layer has a file by that name.
function componentOf(name: string): { name: string; from: string } | undefined {
  if (name === "NuxtLink") return { name: "default", from: here("./module/nuxt-link.ts") };
  if (/^U[A-Z]/.test(name)) return { name: "default", from: `@nuxt/ui/components/${name.slice(1)}.vue` };
  if (existsSync(here(`./node_modules/bmo-ui/app/components/${name}.vue`))) return { name: "default", from: `bmo-ui/app/components/${name}.vue` };

  return undefined;
}

// The bundle's language pack: each locale's words under the locale's code.
function packs(): Record<string, { artifact: { player: unknown } }> {
  const folder = here("./i18n");

  return Object.fromEntries(
    readdirSync(folder)
      .filter((file) => file.endsWith(".json"))
      .sort()
      .map((file) => [basename(file, ".json"), JSON.parse(readFileSync(`${folder}/${file}`, "utf8")) as { artifact: { player: unknown } }])
  );
}

// The bundle's language pack, read once: the bundle build holds the screens to it and the console face answers it.
const WORDS = packs();

// The bundle's words as the console face answers them: each locale's pack under the locale's code.
function words(): Plugin {
  return {
    name: "artifact-words",
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "words.json", source: `${JSON.stringify(WORDS)}\n` });
    }
  };
}

// bmo-ui's audio player script as the packed bmo-ui carries it and the player's words of each locale, in the
// folder `player` beside the bundle's: the service hands them to a hosted page that writes a sound
// (`docs/layout.md`, "The console bundle").
function player(): Plugin {
  let folder = "";

  return {
    name: "artifact-player",
    configResolved(config) {
      folder = resolve(config.root, config.build.outDir, "..", "player");
    },
    closeBundle() {
      const spoken = Object.fromEntries(Object.entries(WORDS).map(([locale, pack]) => [locale, pack.artifact.player]));

      rmSync(folder, { recursive: true, force: true });
      mkdirSync(folder, { recursive: true });
      writeFileSync(`${folder}/bmo-audio-player.js`, readFileSync(here("./node_modules/bmo-ui/dist/bmo-audio-player.js")));
      writeFileSync(`${folder}/words.json`, `${JSON.stringify(spoken)}\n`);
    }
  };
}

export default defineConfig({
  // The module runs in the browser alone: what Nuxt states of the side a file runs on is stated here.
  define: { "import.meta.client": "true", "import.meta.server": "false" },
  plugins: [
    consoleBundle({ name: "artifact", words: WORDS }),
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
    words(),
    player()
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: { entry: here("./module/entry.ts"), formats: ["es"], fileName: () => "bundle.js", cssFileName: "bundle" },
    rolldownOptions: { output: { codeSplitting: false } }
  }
});
