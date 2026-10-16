//! The rows a document is written as (`docs/archive.md`, "`searches`" and "The other five tables"): rows
//! written before documents had kinds settled as the searcher left them, the pictures searches kept taken
//! into the attachments once, a document that answers no question started, and an attachment kept as one
//! row per file.

mod support;

use bmo_artifact::now_ms;
use bmo_artifact::store::documents::{
  Attachment, NewDocument, SEARCHER, keep_attachment, settle, start_document,
};
use serde_json::{Value, json};
use support::scratch::Scratch;

// The prefix a row naming no kind is told an import by, as a deployment sets it.
const IMPORT: &str = "Import: ";

// A search row as a process older than kinds wrote it.
async fn kindless(scratch: &Scratch, query: &str, status: &str, started_at: i64) {
  scratch
    .run(
      "INSERT INTO searches (chat_id, query, started_depth, status, started_at) VALUES (1, $1, 'basic', $2, $3)",
      vec![query.into(), status.into(), started_at.into()],
    )
    .await;
}

// A picture a search kept, as a process older than the attachments wrote it.
async fn kept_picture(scratch: &Scratch, url: &str, path: &str, bytes: i64, kept_at: i64) {
  scratch
    .run(
      "INSERT INTO search_images (search_id, url, path, type, bytes, kept_at) VALUES (1, $1, $2, 'image/avif', $3, $4)",
      vec![url.into(), path.into(), bytes.into(), kept_at.into()],
    )
    .await;
}

// The rows a statement answers, each read from the JSON object it builds.
async fn rows(scratch: &Scratch, sql: &str) -> Vec<Value> {
  scratch
    .texts(sql)
    .await
    .iter()
    .map(|row| serde_json::from_str(row).expect("a JSON row"))
    .collect()
}

async fn attachments(scratch: &Scratch, search: i64) -> Vec<Value> {
  rows(
    scratch,
    &format!(
      "SELECT json_build_object('media', media, 'url', url, 'path', path, 'type', type, 'bytes', bytes, \
       'name', name, 'place', place, 'kept_at', kept_at)::text
         FROM archive_attachments WHERE search_id = {search} ORDER BY id"
    ),
  )
  .await
}

#[tokio::test]
async fn a_row_written_before_kinds_is_settled_as_the_searcher_left_it_and_its_pictures_taken_in() {
  let scratch = Scratch::new().await;

  kindless(&scratch, "silver price", "done", 100).await;
  kindless(&scratch, "Import: https://a.example/post", "done", 200).await;
  kept_picture(&scratch, "https://cdn.example/a.png", "/data/searches/1/images/a.avif", 12, 150).await;

  settle(scratch.db(), Some(IMPORT)).await.expect("settled");

  assert_eq!(
    rows(
      &scratch,
      "SELECT json_build_object('id', id, 'kind', kind, 'author', author)::text FROM searches ORDER BY id"
    )
    .await,
    [
      json!({ "id": 1, "kind": "search", "author": SEARCHER }),
      json!({ "id": 2, "kind": "import", "author": SEARCHER }),
    ]
  );
  assert_eq!(
    attachments(&scratch, 1).await,
    [json!({
      "media": "image",
      "url": "https://cdn.example/a.png",
      "path": "/data/searches/1/images/a.avif",
      "type": "image/avif",
      "bytes": 12,
      "name": "a.avif",
      "place": null,
      "kept_at": 150,
    })]
  );
  // The first table stays as it was for a reader of a copy that has no attachments.
  assert_eq!(
    scratch.texts("SELECT url FROM search_images ORDER BY id").await,
    ["https://cdn.example/a.png"]
  );
}

#[tokio::test]
async fn what_an_older_process_writes_meanwhile_is_taken_in_at_the_next_settling_once() {
  let scratch = Scratch::new().await;

  kindless(&scratch, "silver price", "done", 100).await;
  kept_picture(&scratch, "https://cdn.example/a.png", "/data/searches/1/images/a.avif", 12, 150).await;
  settle(scratch.db(), Some(IMPORT)).await.expect("settled");

  kindless(&scratch, "Import: https://b.example", "open", 300).await;
  kept_picture(&scratch, "https://cdn.example/b.png", "/data/searches/1/images/b.avif", 7, 310).await;
  settle(scratch.db(), Some(IMPORT)).await.expect("settled again");
  settle(scratch.db(), Some(IMPORT)).await.expect("settled a third time");

  assert_eq!(
    scratch.texts("SELECT kind || ' ' || author FROM searches WHERE id = 2").await,
    ["import searcher"]
  );
  assert_eq!(
    scratch
      .texts("SELECT name FROM archive_attachments WHERE search_id = 1 ORDER BY id")
      .await,
    ["a.avif", "b.avif"]
  );
}

#[tokio::test]
async fn where_no_import_prefix_is_set_every_kindless_row_is_a_search() {
  let scratch = Scratch::new().await;

  kindless(&scratch, "Import: https://a.example/post", "done", 200).await;
  settle(scratch.db(), None).await.expect("settled");

  assert_eq!(scratch.texts("SELECT kind FROM searches").await, ["search"]);
}

#[tokio::test]
async fn a_document_that_answers_no_question_is_born_finished_under_its_own_title() {
  let scratch = Scratch::new().await;
  let before = now_ms() / 1000;
  let id = start_document(
    scratch.db(),
    NewDocument {
      chat_id: 7,
      request: "this month's power bill, summed up",
      title: "September power bill",
      kind: "record",
      asker_id: Some(42),
      author: "bmo",
    },
  )
  .await
  .expect("started");
  let row = rows(
    &scratch,
    &format!(
      "SELECT json_build_object('chat_id', chat_id, 'query', query, 'started_depth', started_depth, \
       'ended_depth', ended_depth, 'status', status, 'asker_id', asker_id, 'kind', kind, 'author', author, \
       'title', title, 'documented', documented, 'started_at', started_at, 'finished_at', finished_at)::text
         FROM searches WHERE id = {id}"
    ),
  )
  .await
  .remove(0);

  assert_eq!(row["chat_id"], 7);
  assert_eq!(row["query"], "this month's power bill, summed up");
  assert_eq!(row["started_depth"], "record");
  assert_eq!(row["ended_depth"], "record");
  assert_eq!(row["status"], "done");
  assert_eq!(row["asker_id"], 42);
  assert_eq!(row["kind"], "record");
  assert_eq!(row["author"], "bmo");
  assert_eq!(row["title"], "September power bill");
  assert_eq!(row["documented"], 1);
  assert!(row["started_at"].as_i64().expect("a moment") >= before);
  assert_eq!(row["finished_at"], row["started_at"]);
}

#[tokio::test]
async fn an_attachment_is_one_row_per_file_and_keeping_the_file_again_writes_over_it() {
  let scratch = Scratch::new().await;
  let id = start_document(
    scratch.db(),
    NewDocument {
      chat_id: 7,
      request: "the recording, summed up",
      title: "Meeting",
      kind: "record",
      asker_id: None,
      author: "bmo",
    },
  )
  .await
  .expect("started");
  let recording = "/data/searches/9/attachments/meeting.ogg";

  for (bytes, name) in [(10, "meeting.ogg"), (11, "meeting recording.ogg")] {
    keep_attachment(
      scratch.db(),
      Attachment {
        search_id: id,
        media: "audio",
        url: None,
        path: recording,
        kind: "audio/ogg",
        bytes,
        name,
        place: None,
      },
    )
    .await
    .expect("kept");

    if bytes == 10 {
      keep_attachment(
        scratch.db(),
        Attachment {
          search_id: id,
          media: "image",
          url: Some("https://cdn.example/x.png"),
          path: "/data/searches/9/images/x.avif",
          kind: "image/avif",
          bytes: 4,
          name: "x.avif",
          place: Some(1),
        },
      )
      .await
      .expect("kept");
    }
  }

  let kept: Vec<Value> = attachments(&scratch, id)
    .await
    .into_iter()
    .map(|row| {
      json!({ "media": row["media"], "url": row["url"], "bytes": row["bytes"], "name": row["name"], "place": row["place"] })
    })
    .collect();

  assert_eq!(
    kept,
    [
      json!({ "media": "audio", "url": null, "bytes": 11, "name": "meeting recording.ogg", "place": null }),
      json!({ "media": "image", "url": "https://cdn.example/x.png", "bytes": 4, "name": "x.avif", "place": 1 }),
    ]
  );
}
