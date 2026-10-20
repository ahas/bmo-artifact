//! `bmo-artifact html-read` as a caller runs it: a page read with the pictures inside it written out, and its
//! words cut at the answer ceiling with the line to carry on from named (`docs/archive.md`, "An HTML
//! page").

mod support;

use support::cli::{Ran, cli};

// A one-pixel PNG.
const DOT: &str =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn read(args: &[&str]) -> Ran {
  cli(&[], &[["html-read"].as_slice(), args].concat(), None)
}

#[test]
fn a_page_read_writes_the_pictures_inside_it_out_under_the_names_its_reading_gives_them() {
  let root = tempfile::tempdir().expect("a folder");
  let page = root.path().join("page.html");
  let pictures = root.path().join("pictures");
  let page_path = page.to_str().expect("a path");

  std::fs::write(
    &page,
    format!("<title>Gallery</title><h1>Palette</h1><img src=\"data:image/png;base64,{DOT}\">"),
  )
  .expect("the page is written");

  let written = read(&[page_path, "--pictures", pictures.to_str().expect("a path"), "--stem", "palette"]);

  assert!(written.ok, "{}", written.err);
  assert!(written.out.contains("title: Gallery"), "{}", written.out);
  assert!(
    written
      .out
      .contains(&format!("pictures: 1 inside the page, written to {}", pictures.display())),
    "{}",
    written.out
  );
  assert!(written.out.contains("2\t![](palette-01.png)"), "{}", written.out);
  assert_eq!(
    &std::fs::read(pictures.join("palette-01.png")).expect("the picture is written")[1..4],
    b"PNG"
  );

  let unwritten = read(&[page_path]);

  assert!(unwritten.out.contains("2\t![](picture-01.png)"), "{}", unwritten.out);
  assert!(unwritten.out.contains("pictures: 1 inside the page, not written"), "{}", unwritten.out);

  let climbing = read(&[page_path, "--stem", "../x"]);

  assert!(!climbing.ok);
  assert!(climbing.err.contains("--stem is a short name"), "{}", climbing.err);
}

#[test]
fn a_long_page_read_is_cut_at_the_ceiling_naming_the_line_to_carry_on_from() {
  let root = tempfile::tempdir().expect("a folder");
  let page = root.path().join("long.html");
  let page_path = page.to_str().expect("a path");
  let body: String = (1..=1_500)
    .map(|line| format!("<p>Paragraph number {line} holds a fairly long sentence of plain words.</p>"))
    .collect();

  std::fs::write(&page, body).expect("the page is written");

  let first = read(&[page_path]);
  let next: usize = first
    .out
    .split("Next: --from ")
    .nth(1)
    .and_then(|rest| rest.trim().parse().ok())
    .unwrap_or_else(|| panic!("no line to carry on from in {}", first.out));

  assert!(first.out.chars().count() < 31_000);
  assert!(next > 1);
  assert!(first.out.contains(&format!("{}\tParagraph number {} ", next - 1, next - 1)));

  let rest = read(&[page_path, "--from", &next.to_string()]);

  assert!(rest.out.contains(&format!("{next}\tParagraph number {next} ")), "{}", rest.out);
  assert!(!root.path().join("pictures").exists());

  let nought = read(&[page_path, "--from", "0"]);

  assert!(!nought.ok);
  assert!(nought.err.contains("--from names a line by its number"), "{}", nought.err);
}

#[test]
fn a_file_that_cannot_be_read_is_refused() {
  let gone = read(&["no-such-page.html"]);

  assert_eq!(gone.code, Some(1));
  assert!(gone.err.contains("html-read: no-such-page.html cannot be read."), "{}", gone.err);
}
