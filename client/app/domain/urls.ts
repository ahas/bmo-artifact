// Everything of the console lives under one base path. `$fetch` already knows it; an `<img>`, a
// frame and a link handed out are given the whole path themselves.
export function appUrl(path: string): string {
  const base = useRuntimeConfig().app.baseURL;

  return `${base.replace(/\/$/, "")}${path}`;
}

// The console's root as this page was reached, ending in a slash.
export function appRoot(origin: string): string {
  const base = useRuntimeConfig().app.baseURL;

  return `${origin}${base.endsWith("/") ? base : `${base}/`}`;
}
