import design from "../design/document.json";

// The archive's one document design (`docs/artifact.md`, "How a document is drawn"), read from the
// design file the server reads too: the palette every document is drawn in, in a light and a dark
// scheme. The console's document screen sets the palette through its head and draws a markdown body
// in it; a page carries the server's sheet ahead of its own lines, made out of the same colours.

// One scheme's colours, each by the name its sheet variable takes after `--doc-`.
export type DocumentPalette = Readonly<Record<string, string>>;

export const LIGHT_PALETTE: DocumentPalette = Object.fromEntries(design.palettes.light);
export const DARK_PALETTE: DocumentPalette = Object.fromEntries(design.palettes.dark);

function variables(palette: DocumentPalette): string {
  return Object.entries(palette)
    .map(([name, colour]) => `--doc-${name}:${colour}`)
    .join(";");
}

// The palette for the console, which marks its dark mode with the `dark` class on the root.
export const DOCUMENT_PALETTE = `:root{${variables(LIGHT_PALETTE)}}:root.dark{${variables(DARK_PALETTE)}}`;
