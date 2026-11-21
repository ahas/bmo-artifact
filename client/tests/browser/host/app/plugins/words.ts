import { createI18n } from "vue-i18n";

import en from "../../../../../i18n/en.json";
import ko from "../../../../../i18n/ko.json";

// The screens' own words, as a console gives a bundle its pack: Korean, the reader's language, over English. The
// layer's `common.*` words are a console's own and stand outside this repository, so they are left as their keys.
export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.use(createI18n({ legacy: false, globalInjection: true, locale: "ko", fallbackLocale: "en", messages: { ko, en }, missingWarn: false, fallbackWarn: false }));
});
