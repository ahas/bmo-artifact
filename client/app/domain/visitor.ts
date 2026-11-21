// The token this browser keeps for itself, by which a share page's reads are told apart from another browser's
// (`docs/artifact.md`, "Reads"): drawn once and kept in the browser's storage. A browser that keeps nothing draws a
// new one for each page, so its reads still count once a page.

const KEY = "artifact-visitor";
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

function drawn(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));

  return Array.from(bytes, (byte) => ALPHABET[byte % ALPHABET.length]).join("");
}

/** This browser's visitor token, 16 letters, digits, `-` and `_`. */
export function visitorToken(): string {
  try {
    const kept = localStorage.getItem(KEY);

    if (kept !== null && /^[\w-]{8,64}$/.test(kept)) return kept;

    const token = drawn();

    localStorage.setItem(KEY, token);

    return token;
  } catch {
    return drawn();
  }
}
