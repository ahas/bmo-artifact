//! The synced place (`docs/artifact.md`, "The synced place"): a document's files in a folder on a person's
//! node which their own sync client carries to a library. A file's bytes go and come on the node service's
//! file transfer, relayed to the node's hub under the same judgement as a run; the folders along a path, the
//! stat of a file and a removal are each one command on the same road. A location is the whole path of the
//! file on the node.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::Bytes;
use bmo_node_client::CallError;
use bmo_node_client::NodeClient;
use bmo_node_client::files::{DownloadQuery, UploadQuery};
use bmo_node_client::runs::{Run, RunAsk, RunShell};
use futures_util::StreamExt as _;
use serde_json::{Map, Value};
use sha2::{Digest as _, Sha256};

use super::{Grounds, Kept, Pending, Place, PlaceKind, Reading, field, range_of, text, unknown_field};
use crate::Refusal;
use crate::accounts::Accounts;
use crate::wire::{StorageField, StorageFieldShape};

/// The synced place's name, which its paths begin with.
pub const NAME: &str = "sync";

/// The mark a folder pattern names the owner's login by.
pub const LOGIN: &str = "{login}";
/// The mark a folder pattern names the document's number by, which it must name.
pub const DOCUMENT: &str = "{document}";

/// The kind of the synced place.
pub struct Sync;

// The node path judged: one whole path from a drive or the root, its parts named, none climbing out,
// holding no character a command would have to quote around. A path is written with `/` whatever the
// node's host, and a relative one would stand in one place to the node's commands and another to its
// transfers.
fn node_path(said: &str) -> Result<String, String> {
  if said.contains('\\') {
    return Err("a node path is written with / alone".to_owned());
  }

  if said.contains('\'') {
    return Err("a node path holds no apostrophe".to_owned());
  }

  let head = said.as_bytes();
  let drive = (head.len() >= 3 && head[0].is_ascii_alphabetic() && head[1] == b':' && head[2] == b'/')
    .then(|| head[0] as char);
  let (prefix, body) = match (drive, said.strip_prefix('/')) {
    (Some(letter), _) => (format!("{letter}:/"), &said[3..]),
    (None, Some(body)) => ("/".to_owned(), body),
    (None, None) => return Err("a node path is whole, from a drive or the root".to_owned()),
  };

  let parts: Vec<&str> = body.trim_matches('/').split('/').map(str::trim).collect();

  if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
    return Err("a node path names each of its parts".to_owned());
  }

  Ok(format!("{prefix}{}", parts.join("/")))
}

impl PlaceKind for Sync {
  fn name(&self) -> &'static str {
    NAME
  }

  fn description(&self) -> &'static str {
    "The files are kept in a folder on a person's node which their own sync client carries to the library."
  }

  fn fields(&self) -> Vec<StorageField> {
    vec![
      field("node", StorageFieldShape::Text, true),
      field("folder", StorageFieldShape::Text, true),
    ]
  }

  fn judge(&self, settings: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    unknown_field(settings, &self.fields())?;

    let node = text(settings, "node")?.ok_or("a synced place names its node")?;

    if node.contains('/') || node.contains('\\') {
      return Err("node is the node's name, holding no path".to_owned());
    }

    let pattern = text(settings, "folder")?.ok_or("a synced place names its folder")?;
    let laid = node_path(&pattern)?;

    if !laid.contains(DOCUMENT) {
      return Err(format!("folder names the document's number as {DOCUMENT}"));
    }

    let mut judged = Map::new();

    judged.insert("node".to_owned(), Value::from(node));
    judged.insert("folder".to_owned(), Value::from(laid));

    Ok(judged)
  }

  fn open(
    &self,
    settings: &Map<String, Value>,
    grounds: &Grounds,
    owner: Option<&str>,
  ) -> Result<Arc<dyn Place>, Refusal> {
    let judged = self.judge(settings).map_err(Refusal::failed)?;
    let said = |key: &str| judged.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();

    Ok(Arc::new(Synced {
      node: said("node"),
      folder: said("folder"),
      owner: owner.map(str::to_owned),
      accounts: grounds.accounts.clone(),
      client: NodeClient::new(&grounds.settings.node.address, grounds.settings.node.caller_token.clone())
        .ok(),
      service: NodeClient::new(&grounds.settings.node.address, grounds.settings.node.token.clone()).ok(),
      node_settings: grounds.settings.node.clone(),
    }))
  }
}

/// The node's folder of one document's owner, opened to write and read as them.
pub struct Synced {
  node: String,
  folder: String,
  /// The account the commands and transfers are made for; the node must be their own.
  owner: Option<String>,
  accounts: Accounts,
  /// The node service asked with this service's own caller token, for runs and transfers.
  client: Option<NodeClient>,
  /// The same service asked with its own token, for the one read of whose host the node is.
  service: Option<NodeClient>,
  node_settings: crate::settings::Node,
}

impl Synced {
  fn road(&self) -> Result<(&NodeClient, &NodeClient), Refusal> {
    if self.node_settings.address.trim().is_empty() {
      return Err(Refusal::upstream("this service names no node service for a synced place"));
    }

    match (&self.client, &self.service) {
      (Some(client), Some(service)) => Ok((client, service)),
      _ => Err(Refusal::upstream("the node service's address is no address")),
    }
  }

  fn person(&self) -> Result<&str, Refusal> {
    self
      .owner
      .as_deref()
      .ok_or_else(|| Refusal::conflict("a document with no owner has no person whose node it is"))
  }

  /// The language the node's host takes a command in, read from the node service.
  async fn host(&self) -> Result<RunShell, Refusal> {
    let (_, service) = self.road()?;
    let target = service
      .target(&self.node)
      .await
      .map_err(|why| Refusal::upstream(format!("the node was not reached: {why}")))?;

    Ok(RunShell::of_host(target.shell))
  }

  /// One command on the node, as its person.
  async fn command(&self, shell: RunShell, text: String) -> Result<Run, Refusal> {
    let (client, _) = self.road()?;
    let asked = RunAsk {
      account: self.person()?.to_owned(),
      command: text,
      shell: Some(shell),
      cwd: None,
      env: BTreeMap::new(),
      distro: None,
    };

    client
      .run(&self.node, &asked)
      .await
      .map_err(|why| Refusal::upstream(format!("the node was not reached: {why}")))
  }

  // The command that lays a folder, and every folder above it, open on the node.
  fn making(shell: RunShell, folder: &str) -> String {
    match shell {
      RunShell::Powershell => {
        format!("New-Item -ItemType Directory -Force -Path '{folder}' | Out-Null")
      }
      RunShell::Wsl | RunShell::Linux => format!("mkdir -p -- '{folder}'"),
    }
  }

  // The command that says a file's size and the moment it was written, ending otherwise.
  fn stating(shell: RunShell, path: &str) -> String {
    match shell {
      RunShell::Powershell => format!(
        "if (Test-Path -LiteralPath '{path}' -PathType Leaf) {{ $i = Get-Item -LiteralPath '{path}'; \
         '{{0}} {{1}}' -f $i.Length, $i.LastWriteTimeUtc.Ticks }} else {{ exit 1 }}",
      ),
      RunShell::Wsl | RunShell::Linux => format!("stat -c '%s %Y.%9N' -- '{path}'"),
    }
  }

  // The command that lets a folder go with everything under it, whether it stood or not.
  fn removing(shell: RunShell, path: &str) -> String {
    match shell {
      RunShell::Powershell => {
        format!("Remove-Item -LiteralPath '{path}' -Recurse -Force -ErrorAction SilentlyContinue")
      }
      RunShell::Wsl | RunShell::Linux => format!("rm -rf -- '{path}'"),
    }
  }
}

// The hex of the file's digest, as the hub's transfer checks it.
fn digest(bytes: &[u8]) -> String {
  Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn missed(why: CallError) -> Refusal {
  match why {
    CallError::Answered { status: 404, .. } => Refusal::absent("the file is not in its folder"),
    why => Refusal::upstream(format!("the node was not reached: {why}")),
  }
}

impl Place for Synced {
  fn kind(&self) -> &'static str {
    NAME
  }

  fn folder<'a>(&'a self, document: i64, owner: Option<&'a str>) -> Pending<'a, String> {
    Box::pin(async move {
      let mut laid = self.folder.replace(DOCUMENT, &document.to_string());

      if laid.contains(LOGIN) {
        let owner = owner
          .ok_or_else(|| Refusal::conflict("a document with no owner has no login to lay its folder by"))?;
        let login = self
          .accounts
          .login_of(owner)
          .await?
          .ok_or_else(|| Refusal::upstream(format!("the account service knows no account {owner}")))?;

        laid = laid.replace(LOGIN, &login.replace('/', "_"));
      }

      Ok(laid)
    })
  }

  fn write<'a>(&'a self, at: &'a str, bytes: Vec<u8>) -> Pending<'a, ()> {
    Box::pin(async move {
      self.person()?;
      let (client, _) = self.road()?;
      let shell = self.host().await?;

      if let Some((parent, _)) = at.rsplit_once('/') {
        let made = self.command(shell, Self::making(shell, parent)).await?;

        if made.exit_code != Some(0) {
          return Err(Refusal::upstream(format!(
            "the node could not open the folder: {}",
            made.stderr.trim()
          )));
        }
      }

      let asked = UploadQuery {
        account: self.person()?.to_owned(),
        path: at.to_owned(),
        total: bytes.len() as u64,
        checksum: digest(&bytes),
      };

      client
        .upload_file(&self.node, &asked, Bytes::from(bytes))
        .await
        .map(|_| ())
        .map_err(|why| Refusal::upstream(format!("the file was not written on the node: {why}")))
    })
  }

  fn read<'a>(&'a self, at: &'a str, range: Option<&'a str>) -> Pending<'a, Reading> {
    Box::pin(async move {
      let (client, _) = self.road()?;
      let asked = DownloadQuery {
        account: self.person()?.to_owned(),
        path: at.to_owned(),
      };
      let (total, mut body) = client.download_file(&self.node, &asked).await.map_err(missed)?;
      let mut bytes = Vec::with_capacity(usize::try_from(total).unwrap_or_default());

      while let Some(part) = body.next().await {
        let part = part.map_err(|why| Refusal::upstream(format!("the file was not read: {why}")))?;

        bytes.extend_from_slice(&part);
      }

      let total = bytes.len() as u64;
      let span = range_of(range, total);
      let (from, to) = span.unwrap_or((0, total.saturating_sub(1)));
      let cut = if total == 0 { Vec::new() } else { bytes[from as usize..=to as usize].to_vec() };
      let body = futures_util::stream::once(async { Ok(Bytes::from(cut)) }).boxed();

      Ok(Reading { total, span, body })
    })
  }

  fn exists<'a>(&'a self, at: &'a str) -> Pending<'a, Option<Kept>> {
    Box::pin(async move {
      self.person()?;
      let shell = self.host().await?;
      let said = self.command(shell, Self::stating(shell, at)).await?;

      if said.exit_code != Some(0) {
        return Ok(None);
      }

      let fields: Vec<&str> = said.stdout.split_whitespace().collect();

      match fields.as_slice() {
        [size, moment] => Ok(Some(Kept {
          bytes: size.parse().map_err(|_| {
            Refusal::upstream(format!("the node said a file's size as {:?}", said.stdout.trim()))
          })?,
          version: (*moment).to_owned(),
        })),
        _ => Err(Refusal::upstream(format!(
          "the node did not say the file's size and moment: {:?}",
          said.stdout.trim()
        ))),
      }
    })
  }

  fn delete<'a>(&'a self, at: &'a str) -> Pending<'a, ()> {
    Box::pin(async move {
      self.person()?;
      let shell = self.host().await?;

      self.command(shell, Self::removing(shell, at)).await.map(|_| ())
    })
  }
}
