// What a page's `definePageMeta` stands for in the module: nothing. Nuxt reads the call when it builds
// the pages into a host; the module's routes state the same meta themselves.
export function definePageMeta(_meta: Record<string, unknown>): void {}
