//! The share API (`docs/api.md`, "Share links (HTTP)"; `docs/archive.md`, "The share page"): a document's
//! link made, read back and taken away behind the console's sign-in, and the roads a link opens for whoever
//! holds it. On those roads the token is the whole of the permission and no document number is taken from
//! the request: the record says which document a token opens, so one link is no way to reach another.

use std::collections::HashMap;

use axum::http::{Method, StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};

use super::documents::{
  absent, attachment_id, bad, document_id, now_seconds, picture, reference, with_headers,
};
use super::files::{Sending, attachment};
use crate::documents::ground::{CARD_TYPE, card_picture};
use crate::documents::read::{Library, RefPicture};
use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::store::shares::{self, Closed, SHARE_SPANS, span_seconds};

/// What a shared document's answers may be kept for: nothing past the request, since the link runs out.
pub const NO_STORE: &str = "private, no-store";

fn conflict(code: &str, error: &str) -> Refused {
  Refused::new(StatusCode::CONFLICT, code, error)
}

/// `GET /api/archive/{id}/share`: the live link of one document, read back by the owner; none where it has
/// none, which is also what a link whose time has passed reads as.
pub struct ReadShare;

impl Route for ReadShare {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}/share";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = document_id(&call.path_params().await?)?;
    let library = call.standing().library().await?;
    let share = shares::read(&library.db, id, now_seconds()).await?;

    Ok(answer(StatusCode::OK, json!({ "share": share })))
  }
}

crate::route!(ReadShare);

// The span a body names, refused where it names none the console offers.
fn span_of(body: &Value) -> Result<Option<i64>, Refused> {
  body.get("span").and_then(Value::as_str).and_then(span_seconds).ok_or_else(|| {
    let offered: Vec<&str> = SHARE_SPANS.iter().map(|(key, _)| *key).collect();

    bad("bad_expiry", &format!("choose a span the console offers: {}", offered.join(", ")))
  })
}

/// `POST /api/archive/{id}/share`: a link made for one document, for a span the console offers. The time
/// itself is worked out here, so a caller can neither name an expiry of its own nor a span nobody offers.
pub struct MakeShare;

impl Route for MakeShare {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/api/archive/{id}/share";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = document_id(&call.path_params().await?)?;
    let body = call.body().await?;
    let seconds = span_of(&body)?;
    let by = body
      .get("by")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|by| !by.is_empty())
      .ok_or_else(|| bad("bad_maker", "name who makes the link under by"))?
      .to_owned();
    let library = call.standing().library().await?;
    let now = now_seconds();
    let detail = library
      .detail(id, now)
      .await?
      .ok_or_else(|| absent("document_not_found", "no such document"))?;

    // A link hands out a document, so a search that left none has nothing to hand out.
    if !detail.search.summary.documented {
      return Err(conflict("nothing_to_share", "the document kept nothing to hand out"));
    }

    let share = shares::create(&library.db, id, &by, now, seconds).await?.ok_or_else(|| {
      conflict("already_shared", "the document already has a share link; take it away first")
    })?;

    Ok(answer(StatusCode::OK, json!({ "share": share })))
  }
}

crate::route!(MakeShare);

/// `DELETE /api/archive/{id}/share`: the link of one document taken away. It ends at once, and the row goes
/// with it.
pub struct TakeShare;

impl Route for TakeShare {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/api/archive/{id}/share";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = document_id(&call.path_params().await?)?;
    let library = call.standing().library().await?;
    let removed = shares::remove(&library.db, id).await?;

    Ok(answer(StatusCode::OK, json!({ "removed": removed })))
  }
}

crate::route!(TakeShare);

// The document a road's token opens, and the archive it is read from; refused where the token opens nothing,
// saying which of the two it is: a link nobody was given, or one whose time has passed.
async fn opened(call: &mut Call) -> Result<(Library, i64, HashMap<String, String>), Refused> {
  let params = call.path_params().await?;
  let token = params.get("token").cloned().unwrap_or_default();

  // A token not shaped like one issued here is refused before the archive is asked.
  if !shares::is_share_token(&token) {
    return Err(absent("share_not_found", "no such share link"));
  }

  let library = call.standing().library().await?;

  match shares::opened_by(&library.db, &token, now_seconds()).await? {
    Ok(id) => Ok((library, id, params)),
    Err(Closed::Unknown) => Err(absent("share_not_found", "no such share link")),
    Err(Closed::Expired) => {
      Err(Refused::new(StatusCode::GONE, "share_expired", "the share link has expired"))
    }
  }
}

/// `GET /api/share/{token}`: the one document a link opens, as somebody holding it reads it.
pub struct SharedDocument;

impl Route for SharedDocument {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, _) = opened(&mut call).await?;
    let shared = library
      .shared(id, now_seconds())
      .await?
      .ok_or_else(|| absent("document_not_found", "no such document"))?;

    Ok(with_headers(
      answer(StatusCode::OK, json!(shared)),
      &[(header::CACHE_CONTROL, NO_STORE.to_owned())],
    ))
  }
}

crate::route!(SharedDocument);

/// `GET /api/share/{token}/refs/{ref}/preview`: what a source or a link of a shared document shows of
/// itself, the address read from the record that link opens.
pub struct SharedRefPreview;

impl Route for SharedRefPreview {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}/refs/{ref}/preview";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, params) = opened(&mut call).await?;
    let preview = library
      .ref_preview(id, &reference(&params))
      .await?
      .ok_or_else(|| absent("no_such_source", "no such source"))?;

    Ok(with_headers(
      answer(StatusCode::OK, json!(preview)),
      &[(header::CACHE_CONTROL, "private, max-age=3600".to_owned())],
    ))
  }
}

crate::route!(SharedRefPreview);

/// `GET /api/share/{token}/refs/{ref}/image?as=`: a picture of a shared document, under the same guards as
/// on the console's own road.
pub struct SharedRefImage;

impl Route for SharedRefImage {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}/refs/{ref}/image";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, params) = opened(&mut call).await?;
    let asked = RefPicture::asked(call.query("as").as_deref());
    let image = library
      .ref_image(id, &reference(&params), asked)
      .await?
      .ok_or_else(|| absent("image_unavailable", "the picture is not available"))?;
    let length = image.bytes.len().to_string();

    Ok(picture(image.bytes, &image.kind, length))
  }
}

crate::route!(SharedRefImage);

/// `GET /api/share/{token}/attachments/{attachment}`: one file a shared document keeps beside it, looked up
/// under the document the link opens and never cached past the link.
pub struct SharedAttachment;

impl Route for SharedAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}/attachments/{attachment}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, params) = opened(&mut call).await?;
    let number = attachment_id(&params)?;
    let file = library
      .attachment_file(id, number)
      .await?
      .ok_or_else(|| absent("attachment_not_found", "no such attachment"))?;

    attachment(&file, &Sending::of(&call), NO_STORE).await
  }
}

crate::route!(SharedAttachment);

/// `GET /api/share/{token}/head`: what one link reads as before it is opened, which the share page is drawn
/// with on the server so a chat app previewing the link reads it without running anything.
pub struct SharedHeadRoute;

impl Route for SharedHeadRoute {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}/head";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, params) = opened(&mut call).await?;
    let token = params.get("token").cloned().unwrap_or_default();
    let head = library
      .shared_head(id, &token, now_seconds())
      .await?
      .ok_or_else(|| absent("document_not_found", "no such document"))?;

    Ok(with_headers(
      answer(StatusCode::OK, json!(head)),
      &[(header::CACHE_CONTROL, NO_STORE.to_owned())],
    ))
  }
}

crate::route!(SharedHeadRoute);

/// `GET /api/share/{token}/picture`: the picture a link's preview card shows, the document's first picture
/// as a JPEG a chat app's crawler draws.
pub struct SharedPicture;

impl Route for SharedPicture {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/share/{token}/picture";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let (library, id, _) = opened(&mut call).await?;
    let card = library
      .shared_picture_bytes(id, now_seconds())
      .await?
      .and_then(|bytes| card_picture(&bytes))
      .ok_or_else(|| absent("image_unavailable", "the picture is not available"))?;
    let length = card.len().to_string();

    Ok(picture(card, CARD_TYPE, length))
  }
}

crate::route!(SharedPicture);
