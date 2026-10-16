//! The pictures a run looked at, and the albums the room is sent them in (`docs/archive.md`, "The pictures
//! a search keeps").

use std::collections::HashSet;

use bmo_artifact::verbs::search::gallery::{Picture, albums, looked_at, only_looked_at, pictures};

fn tool_use(id: &str, name: &str, input: serde_json::Value) -> String {
  serde_json::json!({ "type": "assistant", "message": { "content": [{ "type": "tool_use", "id": id, "name": name, "input": input }] } }).to_string()
}

fn tool_result(id: &str, text: &str) -> String {
  serde_json::json!({ "type": "user", "message": { "content": [{ "type": "tool_result", "tool_use_id": id, "content": text }] } }).to_string()
}

#[test]
fn reads_each_picture_with_its_entry_shot_and_source_and_the_entry_it_stands_under() {
  let result = "IMAGE: https://a.example/chart.png :: price trend\n===ITEM===\nNAME: Studio A\nIMAGE: https://a.example/1.jpg :: garden group :: Studio A :: outdoor :: official\nIMAGE: https://a.example/2.jpg :: window\n===ITEM===\n";
  let found = pictures(result);

  assert_eq!(found.len(), 3);
  assert_eq!(found[0].entry, None);
  assert_eq!(
    found[1],
    Picture {
      address: "https://a.example/1.jpg".to_owned(),
      caption: "garden group".to_owned(),
      entry: Some("Studio A".to_owned()),
      shot: Some("outdoor".to_owned()),
      source: Some("official".to_owned()),
    }
  );
  assert_eq!((found[2].entry.as_deref(), found[2].shot.as_deref()), (Some("Studio A"), None));
}

#[test]
fn looked_at_is_what_the_viewer_showed_and_what_a_branch_report_names() {
  let stream = [
    tool_use(
      "v1",
      "mcp__web__view_image",
      serde_json::json!({ "image": "https://a.example/x.jpg?w=1&amp;h=2" }),
    ),
    tool_use("b1", "mcp__web__research_branch", serde_json::json!({ "angle": "reviews" })),
    tool_result("b1", "BRANCH 1 REPORT\nIMAGE: https://b.example/y.jpg :: review photo\nEND OF BRANCH 1"),
    tool_use("f1", "mcp__web__web_fetch", serde_json::json!({ "url": "https://c.example/page" })),
    tool_result("f1", "IMAGE: https://c.example/z.jpg :: a page's own line"),
  ]
  .join("\n");
  let looked = looked_at(&stream).expect("a stream with calls");

  assert!(looked.contains("https://a.example/x.jpg?w=1&h=2"));
  assert!(looked.contains("https://b.example/y.jpg"));
  assert!(!looked.contains("https://c.example/z.jpg"));
  assert_eq!(looked_at("not a stream"), None);
}

#[test]
fn looked_at_are_the_viewers_pictures_however_the_call_is_nested_and_never_a_pages_own() {
  let stream = [
    tool_use(
      "t1",
      "mcp__toolbook__call",
      serde_json::json!({ "tool": "web/view_image", "arguments": { "image": "https://a.example/1.jpg?w=1&amp;h=2" } }),
    ),
    tool_use("t2", "mcp__web__view_image", serde_json::json!({ "image": "https://a.example/2.jpg" })),
    tool_use("t3", "mcp__web__web_fetch", serde_json::json!({ "url": "https://shop.example/" })),
    tool_result("t3", "{\"image\": \"https://shop.example/page.jpg\"}"),
  ]
  .join("\n");
  let mut looked: Vec<String> = looked_at(&stream).expect("a stream with calls").into_iter().collect();

  looked.sort();
  assert_eq!(looked, ["https://a.example/1.jpg?w=1&h=2", "https://a.example/2.jpg"]);
  assert_eq!(looked_at("no events here"), None);
}

#[test]
fn a_report_keeps_only_the_looked_at_pictures() {
  let report = [
    "pictures:",
    "IMAGE: https://a.example/1.jpg?w=1&h=2 :: garden :: A :: outdoor :: official",
    "IMAGE: https://a.example/logo.png :: logo",
    "end",
  ]
  .join("\n");
  let looked = HashSet::from(["https://a.example/1.jpg?w=1&h=2".to_owned()]);

  assert_eq!(
    only_looked_at(&report, &looked).0,
    [
      "pictures:",
      "IMAGE: https://a.example/1.jpg?w=1&h=2 :: garden :: A :: outdoor :: official",
      "end"
    ]
    .join("\n")
  );
  // A stream nothing can be read of says nothing was looked at, and the report is handed back whole.
  assert_eq!(looked_at("not a stream"), None);
}

#[test]
fn takes_out_every_web_picture_nobody_looked_at_and_leaves_a_kept_file() {
  let looked = HashSet::from(["https://a.example/1.jpg".to_owned()]);
  let result = "NAME: A\nIMAGE: https://a.example/1.jpg :: looked at\nIMAGE: https://a.example/logo.png :: never looked at\nIMAGE: /data/searches/1/f.avif :: figure";

  assert_eq!(
    only_looked_at(result, &looked),
    (
      "NAME: A\nIMAGE: https://a.example/1.jpg :: looked at\nIMAGE: /data/searches/1/f.avif :: figure"
        .to_owned(),
      1
    )
  );
}

#[test]
fn sends_one_album_to_each_entry_and_shot_in_the_order_they_stand() {
  let result = [
    "NAME: A",
    "IMAGE: https://x/1.jpg :: garden :: A :: outdoor :: official",
    "IMAGE: https://x/2.jpg :: indoor front :: A :: indoor :: official",
    "IMAGE: https://x/3.jpg :: review garden :: A :: outdoor :: customer",
    "===ITEM===",
    "NAME: B",
    "IMAGE: https://x/4.jpg :: garden :: B :: outdoor :: customer",
  ]
  .join("\n");
  let sent = albums(&result);
  let shape: Vec<(Option<&str>, Vec<&str>)> = sent
    .iter()
    .map(|album| {
      (
        album.caption.as_deref(),
        album.pictures.iter().map(|picture| picture.label.as_str()).collect(),
      )
    })
    .collect();

  assert_eq!(
    shape,
    vec![
      (Some("A · outdoor"), vec!["A · outdoor · official", "A · outdoor · customer"]),
      (Some("A · indoor"), vec!["A · indoor · official"]),
      (Some("B · outdoor"), vec!["B · outdoor · customer"]),
    ]
  );
  assert_eq!(sent[0].pictures[1].url, "https://x/3.jpg");
}

#[test]
fn pictures_with_no_shot_go_together_labelled_by_their_entry_or_their_caption() {
  let result = "IMAGE: https://x/chart.png :: price trend\nNAME: A\nIMAGE: https://x/a.jpg :: front\nNAME: B\nIMAGE: https://x/b.jpg :: front";
  let sent = albums(result);

  assert_eq!(sent.len(), 1);
  assert_eq!(sent[0].caption, None);
  assert_eq!(
    sent[0].pictures.iter().map(|picture| picture.label.as_str()).collect::<Vec<_>>(),
    vec!["price trend", "A", "B"]
  );
}
