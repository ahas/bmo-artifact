//! The document tables and the research profiles as the migrations make them: their columns, keys and
//! indexes as the old archive declares them, the entities that read and write them, and the `migrate` verb
//! that makes them
//! (`docs/layout.md`, "Storage").

mod support;

use std::process::Command;

use bmo_artifact::store::entity::{
  archive_attachments, search_findings, search_images, search_sources, searches, web_search_shares,
};
use bmo_artifact::store::migration::{Migrator, migrate};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use sea_orm_migration::MigratorTrait;
use support::ROOT;
use support::scratch::Scratch;

// The tables of the schema, the migrator's own record among them.
const TABLES: &str = "SELECT table_name::text FROM information_schema.tables
  WHERE table_schema = current_schema() ORDER BY table_name";

// Each column as `<table>.<column> <type>`, then `not null`, then how the database makes it.
const COLUMNS: &str = "SELECT table_name || '.' || column_name || ' ' || data_type
    || CASE WHEN is_nullable = 'NO' THEN ' not null' ELSE '' END
    || CASE WHEN is_identity = 'YES' THEN ' identity ' || lower(identity_generation) ELSE '' END
    || CASE WHEN is_generated = 'ALWAYS' THEN ' generated as ' || generation_expression ELSE '' END
  FROM information_schema.columns
  WHERE table_schema = current_schema() AND table_name <> 'seaql_migrations'
  ORDER BY table_name, ordinal_position";

// Each key as `<table> <kind> (<columns>)`, its columns in the key's order.
const KEYS: &str = "SELECT r.relname || ' '
    || CASE c.contype WHEN 'p' THEN 'primary key' WHEN 'u' THEN 'unique' WHEN 'f' THEN 'foreign key'
       ELSE c.contype::text END
    || ' (' || (SELECT string_agg(a.attname::text, ', ' ORDER BY k.n)
        FROM unnest(c.conkey) WITH ORDINALITY AS k(attnum, n)
        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum) || ')'
  FROM pg_constraint c
  JOIN pg_class r ON r.oid = c.conrelid
  JOIN pg_namespace ns ON ns.oid = r.relnamespace
  WHERE ns.nspname = current_schema() AND r.relname <> 'seaql_migrations'
  ORDER BY 1";

// Each index no key stands on, as `<index> on <table> (<columns>)`.
const INDEXES: &str = "SELECT i.relname || ' on ' || t.relname
    || CASE WHEN x.indisunique THEN ' unique' ELSE '' END
    || ' (' || (SELECT string_agg(a.attname::text, ', ' ORDER BY k.n)
        FROM unnest(x.indkey::int2[]) WITH ORDINALITY AS k(attnum, n)
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = k.attnum) || ')'
  FROM pg_index x
  JOIN pg_class i ON i.oid = x.indexrelid
  JOIN pg_class t ON t.oid = x.indrelid
  JOIN pg_namespace ns ON ns.oid = t.relnamespace
  WHERE ns.nspname = current_schema() AND t.relname <> 'seaql_migrations'
    AND NOT EXISTS (SELECT 1 FROM pg_constraint c WHERE c.conindid = x.indexrelid)
  ORDER BY 1";

fn owned(lines: &[&str]) -> Vec<String> {
  lines.iter().map(|line| (*line).to_owned()).collect()
}

#[tokio::test]
async fn the_migrations_make_the_seven_tables_and_nothing_else() {
  let scratch = Scratch::new().await;

  assert_eq!(
    scratch.texts(TABLES).await,
    owned(&[
      "archive_attachments",
      "research_profiles",
      "seaql_migrations",
      "search_findings",
      "search_images",
      "search_sources",
      "searches",
      "web_search_shares",
    ])
  );
}

#[tokio::test]
async fn the_columns_are_the_old_archives() {
  let scratch = Scratch::new().await;

  assert_eq!(
    scratch.texts(COLUMNS).await,
    owned(&[
      "archive_attachments.id bigint not null identity by default",
      "archive_attachments.search_id bigint not null",
      "archive_attachments.media text not null",
      "archive_attachments.url text",
      "archive_attachments.path text not null",
      "archive_attachments.type text not null",
      "archive_attachments.bytes bigint not null",
      "archive_attachments.name text not null",
      "archive_attachments.place bigint",
      "archive_attachments.kept_at bigint not null",
      "archive_attachments.rowid bigint generated as id",
      "research_profiles.key text not null",
      "research_profiles.cap_ms bigint",
      "research_profiles.synthesis_reserve_ms bigint",
      "research_profiles.effort text",
      "research_profiles.child_role text not null",
      "research_profiles.max_children bigint not null",
      "research_profiles.wait_limit_ms bigint not null",
      "research_profiles.strategy text not null",
      "research_profiles.host_interval_ms bigint not null",
      "research_profiles.engine_interval_ms bigint not null",
      "research_profiles.set_at bigint not null",
      "research_profiles.rowid bigint not null identity by default",
      "search_findings.id bigint not null identity by default",
      "search_findings.search_id bigint not null",
      "search_findings.run_id bigint",
      "search_findings.branch text",
      "search_findings.place bigint not null",
      "search_findings.text text not null",
      "search_findings.rowid bigint generated as id",
      "search_images.id bigint not null identity by default",
      "search_images.search_id bigint not null",
      "search_images.url text not null",
      "search_images.path text not null",
      "search_images.type text not null",
      "search_images.bytes bigint not null",
      "search_images.kept_at bigint not null",
      "search_images.rowid bigint generated as id",
      "search_sources.id bigint not null identity by default",
      "search_sources.search_id bigint not null",
      "search_sources.finding_id bigint",
      "search_sources.url text not null",
      "search_sources.title text",
      "search_sources.supports text",
      "search_sources.mark text",
      "search_sources.rowid bigint generated as id",
      "searches.id bigint not null identity by default",
      "searches.chat_id bigint not null",
      "searches.query text not null",
      "searches.started_depth text not null",
      "searches.ended_depth text",
      "searches.status text not null",
      "searches.started_at bigint not null",
      "searches.finished_at bigint",
      "searches.document text",
      "searches.asker_id bigint",
      "searches.options text",
      "searches.title text",
      "searches.topic text",
      "searches.keywords text",
      "searches.rule text",
      "searches.documented bigint",
      "searches.kind text",
      "searches.author text",
      "searches.rowid bigint generated as id",
      "web_search_shares.share_token text not null",
      "web_search_shares.search_id bigint not null",
      "web_search_shares.created_by text not null",
      "web_search_shares.created_at bigint not null",
      "web_search_shares.expires_at bigint",
      "web_search_shares.rowid bigint not null identity by default",
      "web_search_shares.user_id bigint",
    ])
  );
}

#[tokio::test]
async fn the_keys_are_the_old_archives() {
  let scratch = Scratch::new().await;

  assert_eq!(
    scratch.texts(KEYS).await,
    owned(&[
      "archive_attachments primary key (id)",
      "archive_attachments unique (search_id, path)",
      "research_profiles primary key (key)",
      "research_profiles unique (rowid)",
      "search_findings primary key (id)",
      "search_images primary key (id)",
      "search_images unique (search_id, url)",
      "search_sources primary key (id)",
      "searches primary key (id)",
      "web_search_shares primary key (share_token)",
      "web_search_shares unique (rowid)",
      "web_search_shares unique (search_id)",
    ])
  );
}

#[tokio::test]
async fn the_indexes_are_the_old_archives() {
  let scratch = Scratch::new().await;

  assert_eq!(
    scratch.texts(INDEXES).await,
    owned(&[
      "archive_attachments_search on archive_attachments (search_id)",
      "search_findings_place on search_findings (search_id, place)",
      "search_images_search on search_images (search_id)",
      "search_sources_finding on search_sources (search_id, finding_id)",
    ])
  );
}

#[tokio::test]
async fn a_migrated_database_has_nothing_left_to_run_and_can_be_taken_down() {
  let scratch = Scratch::new().await;

  assert_eq!(migrate(scratch.db()).await.expect("the migration runs"), 0);

  Migrator::down(scratch.db(), None).await.expect("the migration is undone");

  assert_eq!(scratch.texts(TABLES).await, owned(&["seaql_migrations"]));
}

#[tokio::test]
async fn the_migrate_verb_makes_the_schema_on_the_database_the_settings_name() {
  let scratch = Scratch::blank().await;
  let migrate = || {
    Command::new(env!("CARGO_BIN_EXE_bmo-artifact-cli"))
      .arg("migrate")
      .env("BMO_ARTIFACT_DATABASE__URL", scratch.address())
      .current_dir(ROOT)
      .output()
      .expect("the CLI runs")
  };

  let first = migrate();

  assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
  assert_eq!(String::from_utf8_lossy(&first.stdout), "migrations run: 2\n");
  assert_eq!(scratch.texts(TABLES).await.len(), 8);

  let again = migrate();

  assert!(again.status.success(), "{}", String::from_utf8_lossy(&again.stderr));
  assert_eq!(String::from_utf8_lossy(&again.stdout), "migrations run: 0\n");
}

#[tokio::test]
async fn the_migrate_verb_refuses_a_database_it_cannot_reach() {
  let output = Command::new(env!("CARGO_BIN_EXE_bmo-artifact-cli"))
    .arg("migrate")
    .env("BMO_ARTIFACT_DATABASE__URL", "not an address")
    .current_dir(ROOT)
    .output()
    .expect("the CLI runs");

  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  assert!(String::from_utf8_lossy(&output.stderr).starts_with("the archive's database: "));
}

#[tokio::test]
async fn each_entity_writes_and_reads_its_table() {
  let scratch = Scratch::new().await;
  let db = scratch.db();

  let search = searches::ActiveModel {
    chat_id: Set(7),
    query: Set("what was asked".into()),
    started_depth: Set("deep".into()),
    status: Set("done".into()),
    started_at: Set(1_000),
    kind: Set(Some("record".into())),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("a search is written");

  assert_eq!(
    searches::Entity::find_by_id(search.id).one(db).await.expect("read"),
    Some(search.clone())
  );

  let finding = search_findings::ActiveModel {
    search_id: Set(search.id),
    place: Set(1),
    text: Set("a finding".into()),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("a finding is written");

  assert_eq!(
    search_findings::Entity::find_by_id(finding.id).one(db).await.expect("read"),
    Some(finding.clone())
  );

  let source = search_sources::ActiveModel {
    search_id: Set(search.id),
    finding_id: Set(Some(finding.id)),
    url: Set("https://example.com/".into()),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("a source is written");

  assert_eq!(search_sources::Entity::find_by_id(source.id).one(db).await.expect("read"), Some(source));

  let image = search_images::ActiveModel {
    search_id: Set(search.id),
    url: Set("https://example.com/a.png".into()),
    path: Set("images/a.png".into()),
    r#type: Set("image/png".into()),
    bytes: Set(10),
    kept_at: Set(2_000),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("a picture is written");

  assert_eq!(search_images::Entity::find_by_id(image.id).one(db).await.expect("read"), Some(image));

  let attachment = archive_attachments::ActiveModel {
    search_id: Set(search.id),
    media: Set("file".into()),
    path: Set("attachments/a.txt".into()),
    r#type: Set("text/plain".into()),
    bytes: Set(3),
    name: Set("a.txt".into()),
    kept_at: Set(2_000),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("an attachment is written");

  assert_eq!(
    archive_attachments::Entity::find_by_id(attachment.id).one(db).await.expect("read"),
    Some(attachment)
  );

  let share = web_search_shares::ActiveModel {
    share_token: Set("token".into()),
    search_id: Set(search.id),
    created_by: Set("a room".into()),
    created_at: Set(3_000),
    ..Default::default()
  }
  .insert(db)
  .await
  .expect("a share link is written");

  assert_eq!(web_search_shares::Entity::find_by_id("token").one(db).await.expect("read"), Some(share));
  assert_eq!(scratch.count("SELECT rowid FROM searches", vec![]).await, search.id);
}
