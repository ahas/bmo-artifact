// What the host owes this layer beside bmo-ui's own (`docs/layout.md`, "The console plugin bundle"):
// the components the archive screen stands on that the console registers itself. They are declared
// here by the props the screen hands them, so the typecheck holds the screen to what it may ask of
// the host; the host's own components answer them at build, under its own types.
import type { DefineComponent } from "vue";

// Where a picture or a file a text names by its address or its name is shown from, and none where it
// is not one of the document's.
type ImageSource = (href: string) => string | null;

// The console's markdown renderer, drawing a finding in the archive's document design.
type MarkdownText = DefineComponent<{
  text: string;
  kind?: "finding";
  images?: ImageSource;
  files?: ImageSource;
  slugs?: readonly string[];
}>;

// The console's table of contents of one document, which says which heading was pressed.
type DocumentToc = DefineComponent<{
  headings: readonly { slug: string; text: string; depth: number }[];
  active: string | null;
  label?: string;
  onSelect?: (slug: string) => void;
}>;

// The pieces of the console's own title bar.
type TitleBarPiece = DefineComponent<Record<string, never>>;

declare module "vue" {
  interface GlobalComponents {
    MarkdownText: MarkdownText;
    DocumentToc: DocumentToc;
    DeployMark: TitleBarPiece;
    TodoOpenButton: TitleBarPiece;
    UsageMeters: TitleBarPiece;
  }
}

export {};
