// Where a bare web address in a finding ends, the way a chat app finds it, and whether an address is
// one over the web. The server reads addresses by the same rule for what a finding points at; this is
// the console's half of it, for the address the reading turns into a link. A bare address runs to the
// next space or Korean letter, and what the sentence put after it — a full stop, a comma, a closing
// bracket that opened before the address — is the sentence's, not the address's. A pair of brackets
// inside the address, as a wiki page's name carries, stays in it.

// Korean letters: jamo, compatibility jamo, and the syllables with their extensions. An address
// written straight into a Korean sentence ends where the sentence's own letters begin.
const HANGUL = "\\u1100-\\u11ff\\u3130-\\u318f\\ua960-\\ua97f\\uac00-\\ud7af\\ud7b0-\\ud7ff";
const SENTENCE_LETTER = new RegExp(`[\\s<>"'\`${HANGUL}]`);
const TRAILING = /[.,;:!?'"…。、」』》）]+$/;
const BRACKETS = [
  ["(", ")"],
  ["[", "]"],
  ["{", "}"]
] as const;

// A candidate address as it ends, with what the sentence put after it taken off: everything from the
// first space or Korean letter on, the punctuation at its end, and each closing bracket it did not
// open.
export function addressAsWritten(candidate: string): string {
  const letter = SENTENCE_LETTER.exec(candidate);
  let url = (letter === null ? candidate : candidate.slice(0, letter.index)).replace(TRAILING, "");

  for (const [open, close] of BRACKETS) {
    while (url.endsWith(close) && url.split(open).length < url.split(close).length) {
      url = url.slice(0, -1).replace(TRAILING, "");
    }
  }

  return url;
}

// Whether an address is one over the web, read by the platform's own address reader.
export function isWebAddress(url: string): boolean {
  try {
    const parsed = new URL(url);

    return (parsed.protocol === "http:" || parsed.protocol === "https:") && parsed.hostname !== "";
  } catch {
    return false;
  }
}
