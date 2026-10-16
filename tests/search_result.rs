//! What a run's result says of itself, and what the room's session is handed in each ending
//! (`docs/archive.md`, "`searches`").

use bmo_artifact::verbs::search::result::{
  Ending, failure_instruction, imported_instruction, is_answered, is_search_result, lift_narration,
  result_instruction, salvaged_result, stopped_instruction,
};

#[test]
fn a_result_is_written_in_its_structure_and_a_note_under_way_is_not() {
  assert!(is_search_result("TITLE: rates\nSUMMARY: x"));
  assert!(is_search_result("  NAME: a shop"));
  assert!(!is_search_result("Searching for the rate now, will report back."));
  assert!(!is_search_result("TITLE:   "));
}

#[test]
fn only_a_plain_yes_counts_as_answered() {
  assert!(is_answered("ANSWERED: yes\nTITLE: x"));
  assert!(is_answered("answered: YES: met"));
  assert!(!is_answered("ANSWERED: no"));
  assert!(!is_answered("ANSWERED: yesterday"));
  assert!(!is_answered("TITLE: x"));
}

#[test]
fn a_stopped_runs_last_structured_text_is_what_is_salvaged() {
  let texts = vec!["TITLE: first\n".to_owned(), "TITLE: second".to_owned(), "still looking".to_owned()];

  assert_eq!(salvaged_result(&texts), Some("TITLE: second".to_owned()));
  assert_eq!(salvaged_result(&["thinking".to_owned()]), None);
}

#[test]
fn a_narration_is_lifted_out_whole_up_to_the_next_structure_line() {
  let result = "ANSWERED: yes\nTITLE: t\nLISTEN: first words\nsecond line\n\nVERDICT:\nthe pick";
  let (kept, narration) = lift_narration(result);

  assert_eq!(kept, "ANSWERED: yes\nTITLE: t\nVERDICT:\nthe pick");
  assert_eq!(narration.as_deref(), Some("first words\nsecond line"));
  assert_eq!(lift_narration("TITLE: t\nLISTEN:\nSUMMARY: s").1, None);
  assert_eq!(lift_narration("TITLE: t"), ("TITLE: t".to_owned(), None));
}

#[test]
fn a_long_result_asks_for_the_documents_address_first_and_an_unfinished_one_says_so() {
  let long = result_instruction("q", "TITLE: t", Ending::Unanswered, true, Some("https://share.example/d"));

  assert!(long.contains("this link to the whole document"));
  assert!(long.contains("https://share.example/d"));
  assert!(long.contains("did not fully answer"));

  let short = result_instruction("q", "TITLE: t", Ending::Answered, false, None);

  assert!(!short.contains("This result is long"));
  assert!(!short.contains("as far as it got"));
  assert!(
    result_instruction("q", "r", Ending::Stopped, false, None)
      .starts_with("The web search you launched for this chat was stopped by hand")
  );
}

const WRITTEN: &str = "TITLE: dinner nearby\n\nTwo places were checked. The third was not.\n\nSUMMARY: of the two, the first is better.";

fn frame(text: &str) -> String {
  serde_json::json!({ "type": "assistant", "message": { "content": [{ "type": "text", "text": text }] } })
    .to_string()
}

// A run stopped by hand never reaches its result frame, and its stream ends wherever the kill landed.
fn cut_off_stream() -> String {
  [
    serde_json::json!({ "type": "system", "subtype": "init" }).to_string(),
    frame("Splitting the search into three queries."),
    frame(WRITTEN),
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"one more pla"#.to_owned(),
  ]
  .join("\n")
}

// Every text block the run wrote, in the order it wrote them; a line that does not read as JSON is passed by.
fn texts(stream: &str) -> Vec<String> {
  stream
    .lines()
    .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .filter(|event| event["type"] == "assistant")
    .flat_map(|event| event["message"]["content"].as_array().cloned().unwrap_or_default())
    .filter(|block| block["type"] == "text")
    .filter_map(|block| block["text"].as_str().map(str::to_owned))
    .collect()
}

#[test]
fn a_note_about_work_still_going_is_not_an_answer_however_it_is_worded() {
  assert!(!is_search_result(
    "A workflow started in the background over eight angles; the answer follows when it is done."
  ));
  assert!(!is_search_result("The unit watch is running in the background."));
  assert!(!is_search_result(""));
}

#[test]
fn every_shape_the_task_defines_passes_the_set_of_places_included() {
  assert!(is_search_result(WRITTEN));
  assert!(is_search_result(
    "NAME: a coffee place\nMAP: https://map.naver.com/p/search/x\n===ITEM===\nNAME: a roastery"
  ));
  assert!(is_search_result("TITLE: astra\n\nNothing was found."));
}

#[test]
fn a_label_with_nothing_after_it_is_not_structure() {
  assert!(!is_search_result("TITLE:\nSUMMARY:"));
  assert!(!is_search_result(
    "The workflow started. I will put it as TITLE and SUMMARY when it is done."
  ));
}

#[test]
fn is_lifted_out_whole_from_its_line_to_the_verdict_and_the_rest_stands_as_written() {
  let head = "ANSWERED: yes :: done\nTITLE: A model\nTOPIC: a model";
  let body = "VERDICT:\n1. Use it\nSUMMARY:\n1. It fits\nUNCONFIRMED:\n1. none";
  let (result, narration) =
    lift_narration(&format!("{head}\n\nLISTEN:\nThis is what it is.\n\nAnd what to try next.\n\n{body}"));

  assert_eq!(narration.as_deref(), Some("This is what it is.\n\nAnd what to try next."));
  assert_eq!(result, format!("{head}\n\n{body}"));
}

#[test]
fn a_result_without_one_or_with_an_empty_one_carries_none() {
  assert_eq!(
    lift_narration("TITLE: A\nSUMMARY:\n1. B"),
    ("TITLE: A\nSUMMARY:\n1. B".to_owned(), None)
  );
  assert_eq!(
    lift_narration("TITLE: A\nLISTEN:\nSUMMARY:\n1. B"),
    ("TITLE: A\nSUMMARY:\n1. B".to_owned(), None)
  );
}

#[test]
fn what_it_had_written_is_found_in_the_stream_it_left_behind() {
  assert_eq!(salvaged_result(&texts(&cut_off_stream())).as_deref(), Some(WRITTEN));
}

#[test]
fn the_room_gets_that_result_and_is_told_it_was_stopped_there() {
  let instruction = result_instruction("dinner nearby", WRITTEN, Ending::Stopped, false, None);

  assert!(instruction.contains(WRITTEN));
  assert!(instruction.contains("stopped by hand before it finished"));
  assert!(instruction.contains("as far as it got"));
  assert!(instruction.contains("<<SKIP>> is not an option here"));
}

#[test]
fn a_run_stopped_with_only_narration_behind_it_has_left_no_result() {
  let narration = [frame("Starting the search."), frame("The result is not put together yet.")].join("\n");

  assert_eq!(salvaged_result(&texts(&narration)), None);
  assert_eq!(salvaged_result(&[]), None);
}

#[test]
fn with_nothing_written_the_room_still_hears_and_hears_a_stop_rather_than_a_breakdown() {
  let instruction = stopped_instruction("dinner nearby");

  assert!(instruction.contains("dinner nearby"));
  assert!(instruction.contains("was stopped before it had found anything"));
  assert!(instruction.contains("nothing more is coming from it"));
  assert!(instruction.contains("do not answer the question from memory"));
  assert!(!instruction.contains("failed"));
}

#[test]
fn a_search_that_really_failed_is_the_other_line_and_still_says_so() {
  let instruction = failure_instruction("dinner nearby");

  assert!(instruction.contains("dinner nearby"));
  assert!(instruction.contains("did not come back"));
  assert!(instruction.contains("do not answer the question from memory"));
}

#[test]
fn a_long_result_is_asked_for_as_the_link_to_the_whole_document_first_then_a_short_summary() {
  let linked =
    result_instruction("q", WRITTEN, Ending::Answered, true, Some("https://x.test/bmo/share/abc"));
  let unlinked = result_instruction("q", WRITTEN, Ending::Answered, true, None);

  assert!(linked.contains("short summary only"));
  assert!(linked.contains("first line"));
  assert!(
    linked.find("https://x.test/bmo/share/abc").unwrap() < linked.find("short summary only").unwrap()
  );
  assert!(unlinked.contains("short summary only"));
  assert!(!unlinked.contains("https://"));
  assert!(!result_instruction("q", WRITTEN, Ending::Answered, false, None).contains("short summary only"));
}

#[test]
fn a_search_that_answered_says_nothing_about_having_been_stopped_or_thin() {
  let instruction = result_instruction("dinner nearby", WRITTEN, Ending::Answered, false, None);

  assert!(instruction.contains(WRITTEN));
  assert!(instruction.contains("VERDICT part"));
  assert!(!instruction.contains("stopped"));
  assert!(!instruction.contains("as far as it got"));
}

#[test]
fn and_one_the_search_itself_called_unanswered_is_offered_a_deeper_look() {
  let instruction = result_instruction("dinner nearby", WRITTEN, Ending::Unanswered, false, None);

  assert!(instruction.contains(WRITTEN));
  assert!(instruction.contains("did not fully answer the question"));
  assert!(instruction.contains("Offer to look deeper"));
  assert!(!instruction.contains("stopped by hand"));
}

#[test]
fn only_a_plain_yes_reads_as_answered() {
  assert!(is_answered("ANSWERED: yes :: one tool and why\nTITLE: rigging tools\nSUMMARY: Faceit."));
  assert!(is_answered("  answered: YES::price\nTITLE: price"));
  assert!(!is_answered("ANSWERED: no :: price unconfirmed\nTITLE: rigging tools"));
  assert!(!is_answered("ANSWERED: yesterday\nTITLE: rigging tools"));
  assert!(!is_answered("ANSWERED: si\nTITLE: rigging tools"));
  assert!(!is_answered("TITLE: rigging tools\nSUMMARY: Faceit."));
  // The word has to open a line: a sentence that mentions it is not the verdict.
  assert!(!is_answered("TITLE: rigging tools\nThe body says ANSWERED: yes somewhere."));
}

#[test]
fn the_verdict_line_is_not_structure_enough_to_be_a_result_on_its_own() {
  assert!(!is_search_result("ANSWERED: yes :: price"));
  assert_eq!(salvaged_result(&texts(&frame("ANSWERED: yes :: price"))), None);
}

#[test]
fn and_the_rooms_session_is_told_not_to_read_it_out() {
  let instruction = result_instruction(
    "rigging tools",
    "ANSWERED: yes :: one\nTITLE: rigging tools\nSUMMARY: Faceit.",
    Ending::Answered,
    false,
    None,
  );

  assert!(instruction.contains("The ANSWERED and DOCUMENT lines are the search's own words to the runner"));
}

const SOURCED: &str = "NAME: education discount\nThe education price is 8% below list.\nSOURCE: https://apple.com/kr-edu :: discount\n===ITEM===\nTITLE: what to buy\nSUMMARY: buying new with the discount is better.\nSOURCE: https://apple.com/kr/shop/mac :: list price";

#[test]
fn a_result_carrying_those_lines_is_still_a_result() {
  assert!(is_search_result(SOURCED));
  assert_eq!(
    salvaged_result(&texts(&[frame("Starting the search."), frame(SOURCED)].join("\n"))).as_deref(),
    Some(SOURCED)
  );
}

#[test]
fn but_a_page_on_its_own_is_not_one_the_structure_lines_are_what_make_a_result() {
  assert!(!is_search_result("SOURCE: https://apple.com/kr-edu :: discount"));
  assert_eq!(salvaged_result(&texts(&frame("SOURCE: https://example.com/a"))), None);
}

#[test]
fn and_the_room_is_not_read_the_provenance() {
  let instruction = result_instruction("cheapest M4 mac mini", SOURCED, Ending::Answered, false, None);

  assert!(instruction.contains(SOURCED));
  assert!(instruction.contains("provenance kept for the search's own document"));
  assert!(instruction.contains("leave them out of your answer"));
  assert!(instruction.contains("only if somebody asks where something came from"));
}

#[test]
fn an_imported_page_goes_back_as_the_fact_that_it_came_in_with_its_link_and_nothing_of_its_text() {
  let told = imported_instruction(
    "bring in https://r.example/a",
    "Course book",
    6,
    Some("https://ai.example/s/abc"),
    "Archive",
  );

  assert!(told.contains("6 pictures"));
  assert!(told.contains("Its title: Course book"));
  assert!(told.contains("as a document in the Archive"));
  assert!(told.contains("first line"));
  assert!(told.find("https://ai.example/s/abc").unwrap() < told.find("one short line").unwrap());
  assert!(told.contains("Do not summarise the page"));
  assert!(imported_instruction("q", "t", 0, None, "Archive").contains("in the Archive on the console"));
}

#[test]
fn the_rooms_session_answers_only_what_was_asked_and_is_told_nothing_of_pictures_where_there_are_none() {
  let instruction = result_instruction(
    "silver",
    "TITLE: silver\nSUMMARY:\n1. 63.74 dollars",
    Ending::Answered,
    false,
    None,
  );

  assert!(instruction.contains("with what was asked and nothing beside it"));
  assert!(instruction.contains("which rule the picks were filtered by"));
  assert!(!instruction.contains("photo tool"));
}

#[test]
fn the_session_is_handed_each_album_as_the_photo_call_that_sends_it() {
  let result = "TITLE: studios\n===ITEM===\nNAME: A\nIMAGE: /data/a.avif :: garden :: A :: outdoor :: official\nIMAGE: https://x/b.jpg :: indoor :: A :: indoor :: customer\n===ITEM===";
  let instruction = result_instruction("studios", result, Ending::Answered, false, None);

  assert!(instruction.contains("one call to each album, in the order listed"));
  assert!(instruction.contains(
    r#"ALBUM 1: {"pictures":[{"url":"/data/a.avif","label":"A · outdoor · official"}],"caption":"A · outdoor"}"#
  ));
  assert!(instruction.contains(
    r#"ALBUM 2: {"pictures":[{"url":"https://x/b.jpg","label":"A · indoor · customer"}],"caption":"A · indoor"}"#
  ));
}

#[test]
fn the_session_passes_a_pictures_name_through_as_it_was_written_kept_file_or_address() {
  let instruction =
    result_instruction("soup", "TITLE: soup\nIMAGE: /data/s.avif :: soup", Ending::Answered, false, None);

  assert!(
    instruction.contains("either a file already kept on this machine or the address it was found at")
  );
  assert!(instruction.contains("never rewrite one"));
}

#[test]
fn the_session_says_nothing_about_a_rule_where_the_result_named_none() {
  let instruction = result_instruction("silver", "TITLE: silver", Ending::Answered, false, None);

  assert!(
    instruction
      .contains("Most searches name none, and where none is named say nothing about any rule at all")
  );
}

#[test]
fn the_session_passes_reviews_through_as_they_were_written() {
  let instruction = result_instruction("soup", "TITLE: soup", Ending::Answered, false, None);

  assert!(instruction.contains("REVIEW lines"));
  assert!(instruction.contains("rather than summarising them"));
}
