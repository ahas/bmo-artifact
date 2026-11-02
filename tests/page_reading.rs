//! How a document is read without the database (`docs/artifact.md`): a page read for its title, its
//! description and its words, the pictures inside it taken out, a markdown body's description and the links
//! it makes, a file told by its bytes, and a kept file's name.

use bmo_artifact::documents::filing::links_of;
use bmo_artifact::documents::keeper::{KEPT_PICTURE, attachment_kind, file_name};
use bmo_artifact::documents::page_reading::{
  DESCRIPTION_LENGTH, PICTURE_STEM, markdown_description, page_description, read_html,
};
use bmo_artifact::helpers::picture::{self, raster_type};

#[test]
fn a_page_is_read_for_its_title_its_description_and_its_words() {
  let reading = read_html(
    "<!doctype html><title> Compare  canvas </title><style>h1{color:teal}</style><h1>Compare canvas</h1><p>Two plans &amp; side<b>by</b>side</p><ul><li>one</li></ul><img alt=\"x\" src=\"a.png\">",
    PICTURE_STEM,
  );

  assert_eq!(reading.title.as_deref(), Some("Compare canvas"));
  // A space stands where an element opens, as the console's reading puts one, and none where it closes.
  assert_eq!(reading.lines, ["# Compare canvas", "Two plans & side byside", "- one", "![x](a.png)"]);
  assert_eq!(page_description(&reading).as_deref(), Some("Two plans & side byside one"));
}

#[test]
fn a_picture_inside_the_page_is_taken_out_under_the_stem_and_one_outside_it_stays_where_it_points() {
  let reading = read_html(
    "<p>before</p><img alt=\"mark\" src=\"data:image/svg+xml,%3Csvg%2F%3E\"><img src=\"data:image/x-icon;base64,AAE=\"><img src=\"data:text/plain,hi\"><img src=\"far.png\">",
    "plan",
  );

  assert_eq!(
    reading.lines,
    [
      "before",
      "![mark](plan-01.svg)",
      "![](plan-02.x-icon)",
      "![](data:text/plain,hi)",
      "![](far.png)"
    ]
  );
  assert_eq!(reading.pictures[0].bytes, b"<svg/>");
  assert_eq!(reading.pictures[0].kind, "image/svg+xml");
  assert_eq!(reading.pictures[1].bytes, [0, 1]);
}

#[test]
fn a_description_meta_is_what_the_page_says_of_itself_and_a_long_reading_is_cut_at_a_space() {
  let said = read_html("<meta name=\"Description\" content=\"  What   it is \"><p>words</p>", PICTURE_STEM);

  assert_eq!(page_description(&said).as_deref(), Some("What it is"));

  let long = read_html(&format!("<p>{}</p>", "word ".repeat(60)), PICTURE_STEM);
  let cut = page_description(&long).expect("a description");

  assert!(cut.ends_with("word…"));
  assert!(cut.chars().count() <= DESCRIPTION_LENGTH + 1);
}

#[test]
fn a_file_is_told_by_its_bytes_first_and_its_name_where_they_do_not_settle_it() {
  assert_eq!(attachment_kind(b"OggS\0\x02", "voice.ogg"), ("audio", "audio/ogg".to_owned()));
  assert_eq!(attachment_kind(b"OggS\0\x02", "clip.ogv"), ("video", "video/ogg".to_owned()));
  assert_eq!(attachment_kind(b"%PDF-1.7", "x.bin"), ("file", "application/pdf".to_owned()));
  assert_eq!(attachment_kind(b"<!DOCTYPE html><p>", "page.txt"), ("page", "text/html".to_owned()));
  assert_eq!(attachment_kind(b"a,b\n1,2\n", "table.csv"), ("text", "text/csv".to_owned()));
  assert_eq!(attachment_kind(b"\0\x01\x02", "blob"), ("file", "application/octet-stream".to_owned()));
}

#[test]
fn a_kept_files_name_climbs_out_of_nothing() {
  assert_eq!(file_name("../../etc/passwd"), "_.._etc_passwd");
  assert_eq!(file_name("..."), "file");
  assert_eq!(file_name("report v2.pdf"), "report_v2.pdf");
}

#[test]
fn a_markdown_bodys_description_is_its_opening_prose_without_headings_pictures_or_code() {
  let body = "# Title\n\n![cover](cover.png)\n\nThe *first* words, with `code` in them.\n\n```\nnot this\n```\n\nThen more.";

  assert_eq!(
    markdown_description(body).as_deref(),
    Some("The first words, with code in them. Then more.")
  );
  assert_eq!(markdown_description("# Only a heading"), None);

  let cut = markdown_description(&"word ".repeat(80)).expect("a description");

  assert!(cut.ends_with("word…"));
  assert!(cut.chars().count() <= DESCRIPTION_LENGTH + 1);
}

#[test]
fn a_body_refers_to_each_web_page_it_links_once_in_its_order() {
  let body = "See [the plan](https://example.com/plan) and <https://example.org/>, the [plan again](https://example.com/plan), \
              a [file](figure.png) and [mail](mailto:someone@example.com).";

  assert_eq!(
    links_of(body),
    [
      ("https://example.com/plan".to_owned(), Some("the plan".to_owned())),
      ("https://example.org/".to_owned(), Some("https://example.org/".to_owned())),
    ]
  );
}

#[test]
fn a_raster_picture_is_kept_as_an_avif_and_anything_else_is_not_a_picture() {
  let mut png = Vec::new();

  image::RgbImage::from_pixel(40, 30, image::Rgb([200, 40, 40]))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .expect("a png is written");

  assert_eq!(raster_type(&png), Some("image/png"));
  assert_eq!(raster_type(b"OggS\0\x02"), None);

  let kept = picture::archived(&png, KEPT_PICTURE).expect("a png is kept");

  assert_eq!(raster_type(&kept), Some("image/avif"));
  assert_eq!(picture::archived(b"not a picture", KEPT_PICTURE), None);
}
