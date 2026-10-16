//! The address a long answer is handed with (`docs/archive.md`, "`searches`").

use bmo_artifact::verbs::search::share::{LONG_RESULT_CHARS, is_long};

#[test]
fn a_result_past_a_screen_is_long_and_one_within_it_is_not() {
  assert!(!is_long(&"a".repeat(LONG_RESULT_CHARS)));
  assert!(is_long(&"a".repeat(LONG_RESULT_CHARS + 1)));
}
