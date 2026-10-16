//! The pictures a search keeps (`docs/archive.md`, "The pictures a search keeps") and the page reader they
//! come through (`docs/api.md`, "Read a page and fetch a picture"): the IMAGE lines read, each address fetched
//! through a fake page reader, and the copies written beside the search with their rows.

mod support;

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bmo_artifact::clients::tool_host::{
  Captured, DrawnFigure, PageReader, Raster, Reached, ReadPage, ReadReference, Reading,
};
use bmo_artifact::verbs::search::images::{
  CapturedFigure, Keeping, image_addresses, kept_path, standalone_svg, with_kept,
};
use bmo_artifact::verbs::search::record::{NewSearch, kept_pictures, start_search};
use bmo_artifact::verbs::search::request::Options;
use bmo_artifact::verbs::search::web::{Figure, WebLayer};
use support::scratch::Scratch;

// A small PNG, as a server would send one.
fn png() -> Vec<u8> {
  let mut bytes = Cursor::new(vec![]);

  image::RgbImage::from_pixel(4, 3, image::Rgb([200, 40, 40]))
    .write_to(&mut bytes, image::ImageFormat::Png)
    .unwrap();
  bytes.into_inner()
}

// The web as a test has it: one address with a picture behind it, one with a page that is no picture, one
// lesson page drawing two figures of which the first captured, and nothing anywhere else.
struct FakeReader;

impl PageReader for FakeReader {
  fn picture<'a>(&'a self, address: &'a str) -> Reached<'a, Raster> {
    Box::pin(async move {
      match address {
        "https://img.example/a.jpg" => Ok(Raster {
          kind: "application/octet-stream".to_owned(),
          bytes: png(),
        }),
        "https://img.example/page.html" => Ok(Raster {
          kind: "image/png".to_owned(),
          bytes: b"<!doctype html><title>a page</title>".to_vec(),
        }),
        _ => Err("the address answered 404".to_owned()),
      }
    })
  }

  fn page<'a>(&'a self, address: &'a str) -> Reached<'a, Reading> {
    Box::pin(async move {
      if address != "https://pages.example/lesson" {
        return Err("the page did not open".to_owned());
      }

      Ok(Reading {
        page: ReadPage {
          url: "https://pages.example/lesson#top".to_owned(),
          title: "Course book".to_owned(),
          keywords: vec!["probability".to_owned()],
          body: "SECTION: 1 data".to_owned(),
          references: vec![ReadReference {
            text: "the paper".to_owned(),
            url: "https://papers.example/grpo".to_owned(),
          }],
          figures: vec![
            DrawnFigure {
              n: 1,
              caption: "figure 1".to_owned(),
              svg: Some("<svg width=\"1\"/>".to_owned()),
            },
            DrawnFigure {
              n: 2,
              caption: "figure 2".to_owned(),
              svg: None,
            },
          ],
        },
        captured: vec![Captured { n: 1, png: png() }],
      })
    })
  }
}

fn web() -> WebLayer {
  WebLayer::new(Arc::new(FakeReader))
}

#[test]
fn reads_the_web_addresses_a_result_names_each_once() {
  let result = "IMAGE: https://img.example/a.jpg :: front\nIMAGE: https://img.example/a.jpg\n  IMAGE: http://img.example/b.png";

  assert_eq!(
    image_addresses(result),
    vec!["https://img.example/a.jpg".to_owned(), "http://img.example/b.png".to_owned()]
  );
}

#[test]
fn passes_over_a_kept_file_and_a_scheme_that_is_not_the_web() {
  assert!(
    image_addresses("IMAGE: /data/searches/1/images/x.avif :: kept\nIMAGE: javascript:alert(1) :: fake")
      .is_empty()
  );
}

#[test]
fn names_a_kept_picture_by_its_addresss_digest_under_the_searchs_own_folder() {
  let path = kept_path(Path::new("/data/searches"), 35, "https://img.example/a.jpg");
  let name = path.file_name().unwrap().to_string_lossy().into_owned();

  assert!(path.starts_with("/data/searches/35/images"));
  assert_eq!(name.len(), 32 + ".avif".len());
  assert_eq!(path, kept_path(Path::new("/data/searches"), 35, "https://img.example/a.jpg"));
}

#[test]
fn hands_the_session_the_kept_file_in_place_of_each_address_kept_and_leaves_the_rest() {
  let kept = BTreeMap::from([("https://img.example/a.jpg".to_owned(), "/data/a.avif".to_owned())]);
  let result = "NAME: shop\nIMAGE: https://img.example/a.jpg :: front\nIMAGE: https://img.example/b.jpg :: back\nIMAGE: https://img.example/a.jpg";

  assert_eq!(
    with_kept(result, &kept),
    "NAME: shop\nIMAGE: /data/a.avif :: front\nIMAGE: https://img.example/b.jpg :: back\nIMAGE: /data/a.avif"
  );
  assert_eq!(with_kept(result, &BTreeMap::new()), result);
}

#[test]
fn gives_an_inline_svg_the_namespace_a_file_standing_alone_needs() {
  assert_eq!(
    standalone_svg("<svg width=\"1\"/>"),
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="1"/>"#
  );
  assert_eq!(standalone_svg(r#"<svg xmlns="x"/>"#), r#"<svg xmlns="x"/>"#);
}

// the page reader

#[tokio::test]
async fn a_picture_is_typed_by_its_own_bytes_and_what_is_no_picture_is_refused() {
  let web = web();
  let picture = web.picture("https://img.example/a.jpg").await.unwrap();

  assert_eq!(picture.kind, "image/png");
  assert_eq!(picture.bytes, png());
  assert_eq!(
    web.picture("https://img.example/page.html").await,
    Err("what came is not a picture".to_owned())
  );
  assert_eq!(
    web.picture("https://img.example/gone.jpg").await,
    Err("the address answered 404".to_owned())
  );
}

#[tokio::test]
async fn a_page_carries_each_figures_capture_and_none_for_one_that_did_not_capture() {
  let page = web().page("https://pages.example/lesson").await.unwrap();

  assert_eq!(page.url, "https://pages.example/lesson#top");
  assert_eq!(page.references[0].url, "https://papers.example/grpo");
  assert_eq!(
    page.figures,
    vec![
      Figure {
        n: 1,
        caption: "figure 1".to_owned(),
        png: Some(png()),
        svg: Some("<svg width=\"1\"/>".to_owned()),
      },
      Figure {
        n: 2,
        caption: "figure 2".to_owned(),
        png: None,
        svg: None,
      },
    ]
  );
  assert_eq!(web().page("https://pages.example/other").await, Err("the page did not open".to_owned()));
}

// the copies kept

async fn a_search(scratch: &Scratch) -> i64 {
  start_search(
    scratch.db(),
    &NewSearch {
      chat_id: 7,
      query: "studios nearby",
      depth: "basic",
      asker_id: None,
      options: &Options::default(),
      kind: None,
    },
  )
  .await
  .unwrap()
}

#[tokio::test]
async fn keeps_each_picture_the_result_names_once_and_leaves_the_rest_their_addresses() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = a_search(&scratch).await;
  let keeping = Keeping {
    db: scratch.db(),
    search_id,
    artifacts: artifacts.path(),
    deadline: Duration::from_secs(30),
  };
  let result = "NAME: A\nIMAGE: https://img.example/a.jpg :: front\nIMAGE: https://img.example/page.html :: a page\nIMAGE: https://img.example/gone.jpg :: gone\nIMAGE: https://img.example/a.jpg :: again\nIMAGE: https://img.example/before.jpg :: kept before";
  let already =
    BTreeMap::from([("https://img.example/before.jpg".to_owned(), "/kept/before.avif".to_owned())]);
  let kept = keeping.result_pictures(&web(), result, already).await;
  let copy = kept_path(artifacts.path(), search_id, "https://img.example/a.jpg");

  assert_eq!(
    kept,
    BTreeMap::from([
      ("https://img.example/a.jpg".to_owned(), copy.to_string_lossy().into_owned()),
      ("https://img.example/before.jpg".to_owned(), "/kept/before.avif".to_owned()),
    ])
  );
  assert!(std::fs::read(&copy).unwrap().get(4..12) == Some(b"ftypavif".as_slice()));

  use std::os::unix::fs::PermissionsExt;
  assert_eq!(std::fs::metadata(&copy).unwrap().permissions().mode() & 0o777, 0o600);

  assert_eq!(
    kept_pictures(scratch.db(), search_id).await.unwrap(),
    BTreeMap::from([("https://img.example/a.jpg".to_owned(), copy.to_string_lossy().into_owned())])
  );
}

#[tokio::test]
async fn keeps_a_captured_figure_with_the_drawing_it_was_made_from_beside_it() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = a_search(&scratch).await;
  let keeping = Keeping {
    db: scratch.db(),
    search_id,
    artifacts: artifacts.path(),
    deadline: Duration::from_secs(30),
  };
  let address = "https://pages.example/lesson#figure-1";
  let kept = keeping
    .captured(vec![CapturedFigure {
      address: address.to_owned(),
      png: png(),
      svg: Some("<svg width=\"1\"/>".to_owned()),
    }])
    .await;
  let copy = kept_path(artifacts.path(), search_id, address);

  assert_eq!(kept, BTreeMap::from([(address.to_owned(), copy.to_string_lossy().into_owned())]));
  assert_eq!(
    std::fs::read_to_string(copy.with_extension("svg")).unwrap(),
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="1"/>"#
  );
  assert_eq!(kept_pictures(scratch.db(), search_id).await.unwrap(), kept);
}
