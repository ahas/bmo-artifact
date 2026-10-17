import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { parse } from "vue/compiler-sfc";

// What a host installs is the package packed from client/, holding only the files its manifest names
// (`docs/layout.md`, "How a host takes it"). So every file the package carries reaches only files the
// package carries: an import that left client/, or reached a folder the package leaves out, would
// resolve in this checkout and fail in a host's install. The imports are read the way Bun reads a
// module, and a component's script the way Vue's own compiler splits it out of the component.

const ROOT = resolve(import.meta.dir, "..");
const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as { files: string[]; main: string };
const transpiler = new Bun.Transpiler({ loader: "ts" });

// Every file under one entry the package carries.
function filesUnder(entry: string): string[] {
  const path = join(ROOT, entry);

  return statSync(path).isDirectory() ? readdirSync(path, { recursive: true, encoding: "utf8" }).map((one) => join(path, one)).filter((one) => statSync(one).isFile()) : [path];
}

const carried = manifest.files.flatMap(filesUnder);

// The scripts of one file, as the module it is read as.
function scriptsOf(path: string): string[] {
  if (path.endsWith(".vue")) {
    const { descriptor } = parse(readFileSync(path, "utf8"), { filename: path });

    return [descriptor.script?.content, descriptor.scriptSetup?.content].filter((one): one is string => one !== undefined);
  }

  return /\.(?:ts|js|mjs)$/.test(path) ? [readFileSync(path, "utf8")] : [];
}

// Whether a path stands inside one of the entries the package carries.
function isCarried(path: string): boolean {
  const inside = relative(ROOT, path);

  return !inside.startsWith("..") && manifest.files.some((entry) => inside === entry || inside.startsWith(`${entry}${sep}`));
}

describe("the package packed from client/", () => {
  test("carries the Nuxt configuration a host extends, the layer's files and the design files", () => {
    expect(manifest.main).toBe("./nuxt.config.ts");
    expect(manifest.files).toEqual(expect.arrayContaining(["nuxt.config.ts", "app", "shared", "design", "patches"]));
    expect(carried.map((path) => relative(ROOT, path))).toEqual(expect.arrayContaining([join("design", "document.json"), join("design", "hosted-page.json"), join("app", "pages", "archive", "index.vue")]));
  });

  // bmo-ui is a peer the host supplies: a dependency a package names by a file inside itself is not one
  // a host's install can resolve.
  test("leaves the bundle's own tests, typecheck declarations and copy of bmo-ui out", () => {
    expect(carried.some((path) => ["tests", "host", "vendor"].some((left) => relative(ROOT, path).startsWith(`${left}${sep}`)))).toBe(false);
  });

  test("reaches nothing by a relative import that the package does not carry", () => {
    const reaching = carried.flatMap((path) =>
      scriptsOf(path)
        .flatMap((script) => transpiler.scanImports(script))
        .map((found) => found.path)
        .filter((target) => target.startsWith("."))
        .map((target) => resolve(dirname(path), target))
        .filter((target) => !isCarried(target))
        .map((target) => `${relative(ROOT, path)} -> ${relative(ROOT, target)}`)
    );

    expect(reaching).toEqual([]);
  });

  // The check above reads what it should: an import out of the package is named.
  test("names an import that leaves the package", () => {
    const outside = transpiler.scanImports('import design from "../../design/document.json";').map((found) => resolve(join(ROOT, "shared"), found.path));

    expect(outside.map(isCarried)).toEqual([false]);
  });
});
