// The finding reading, fetched once for the whole screen. The first finding to be shown asks for it
// and every one after that is built the moment it stands, so a document of fifty findings downloads
// it once and waits for it once.
//
// Whether this is a browser is asked outright. The answer is known while the build is being made, so
// the server's copy of this function ends at that line and the reading is in no bundle but the
// browser's.

type Reading = typeof import("./finding-markdown");

let loaded: Reading | null = null;
let loading: Promise<Reading | null> | null = null;

// The reading when it has already arrived, and null until it has.
export function findingReading(): Reading | null {
  return loaded;
}

// The reading, fetched if nobody has asked for it yet. Null on the server, and null where the fetch
// failed; a failed fetch is forgotten so the next finding asks again.
export function loadFindingReading(): Promise<Reading | null> {
  if (!import.meta.client) {
    return Promise.resolve(null);
  }

  loading ??= import("./finding-markdown").then(
    (reading) => (loaded = reading),
    () => {
      loading = null;

      return null;
    }
  );

  return loading;
}
