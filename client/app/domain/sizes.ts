import { countLocale } from "bmo-ui/app/domain/numbers";

// How big a kept file is, in the unit that reads at a glance, its figures in the host's locale.
export function sizeText(bytes: number): string {
  const locale = countLocale();

  if (bytes < 1024) return `${bytes.toLocaleString(locale)} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024).toLocaleString(locale)} KB`;

  return `${(bytes / 1024 / 1024).toLocaleString(locale, { maximumFractionDigits: 1 })} MB`;
}
