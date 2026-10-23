import { defineNuxtLink } from "nuxt/app";

// The console's link component, made from its own Nuxt app: a screen's `NuxtLink` resolves here, since a
// console registers no component for a bundle.
export default defineNuxtLink({ componentName: "NuxtLink" });
