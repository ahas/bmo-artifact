//! The binary as a caller runs it: from the repository's root, so its settings are the committed
//! ones, with the settings a test gives in the environment over them and none the test's own environment
//! carries.

use std::io::Write;
use std::process::{Command, Stdio};

use super::ROOT;

/// How a run ended.
pub struct Ran {
  pub ok: bool,
  pub code: Option<i32>,
  pub out: String,
  pub err: String,
}

/// The binary run on `args` with the settings in `env` and `stdin` on its standard input.
pub fn cli(env: &[(&str, String)], args: &[&str], stdin: Option<&str>) -> Ran {
  let mut command = Command::new(env!("CARGO_BIN_EXE_bmo-artifact"));

  // The service's settings, the agent a harness's turn runs as, which the archive verb files under where no filer
  // is named, and the account a supervisor's program works for, which it acts for.
  for (name, _) in std::env::vars().filter(|(name, _)| {
    name.starts_with("BMO_ARTIFACT_") || name == "BMO_TURN_AGENT" || name == "BMO_OWNER_ACCOUNT"
  }) {
    command.env_remove(name);
  }

  command
    .args(args)
    .envs(env.iter().map(|(name, value)| (*name, value)))
    .current_dir(ROOT)
    .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

  let mut child = command.spawn().expect("the CLI starts");

  if let Some(text) = stdin {
    let mut pipe = child.stdin.take().expect("a standard input");

    pipe.write_all(text.as_bytes()).expect("the standard input is written");
  }

  let done = child.wait_with_output().expect("the CLI ends");

  Ran {
    ok: done.status.success(),
    code: done.status.code(),
    out: String::from_utf8_lossy(&done.stdout).into_owned(),
    err: String::from_utf8_lossy(&done.stderr).into_owned(),
  }
}
