//! A picture's type read off its bytes, and a picture as a document of the archive keeps it, copied from
//! bmo-picture (`docs/inventory.md`, section 5). It reads JPEG, PNG, WebP, GIF, TIFF, AVIF and SVG, and
//! nothing else.

mod avif;
mod svg;

use std::io::Cursor;

use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

/// Why bytes are no picture this module reads.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct NotAPicture(pub String);

fn failed(why: impl std::fmt::Display) -> NotAPicture {
  NotAPicture(why.to_string())
}

/// The type a picture's own first bytes say it is, of those a page serves.
pub fn raster_type(bytes: &[u8]) -> Option<&'static str> {
  let starts = |values: &[u8]| bytes.starts_with(values);
  let text = |offset: usize, value: &str| bytes.get(offset..offset + value.len()) == Some(value.as_bytes());

  if starts(&[0xff, 0xd8, 0xff]) {
    Some("image/jpeg")
  } else if starts(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) {
    Some("image/png")
  } else if text(0, "GIF87a") || text(0, "GIF89a") {
    Some("image/gif")
  } else if text(0, "RIFF") && text(8, "WEBP") {
    Some("image/webp")
  } else if text(4, "ftyp") && (text(8, "avif") || text(8, "avis")) {
    Some("image/avif")
  } else if starts(&[0x00, 0x00, 0x01, 0x00]) && bytes.len() > 6 {
    Some("image/x-icon")
  } else {
    None
  }
}

// The picture bytes hold, and whether they ask to be turned to stand upright.
fn decode(bytes: &[u8], upright: bool) -> Result<DynamicImage, NotAPicture> {
  match image::guess_format(bytes) {
    Ok(
      format @ (ImageFormat::Jpeg
      | ImageFormat::Png
      | ImageFormat::Gif
      | ImageFormat::WebP
      | ImageFormat::Tiff),
    ) => {
      let mut decoder =
        ImageReader::with_format(Cursor::new(bytes), format).into_decoder().map_err(failed)?;
      let orientation = decoder.orientation().map_err(failed)?;
      let mut picture = DynamicImage::from_decoder(decoder).map_err(failed)?;

      if upright {
        picture.apply_orientation(orientation);
      }

      Ok(picture)
    }
    _ if bytes.get(4..8) == Some(b"ftyp") => avif::decode(bytes),
    _ if svg::is_svg(bytes) => svg::decode(bytes),
    _ => Err(failed("unsupported image format")),
  }
}

/// The picture bytes hold, turned upright as they ask: a raster picture, an AVIF or an SVG drawing.
pub fn upright(bytes: &[u8]) -> Result<DynamicImage, NotAPicture> {
  decode(bytes, true)
}

/// The size a picture is brought inside a square of `limit` pixels at, never larger than it stands.
pub fn inside(width: u32, height: u32, limit: u32) -> (u32, u32) {
  if width <= limit && height <= limit {
    return (width, height);
  }

  let scale = f64::from(limit) / f64::from(width.max(height));
  let scaled = |side: u32| ((f64::from(side) * scale).round() as u32).clamp(1, limit);

  (scaled(width), scaled(height))
}

/// How a picture kept beside a document of the archive is written: inside how many pixels, at what AVIF
/// quality and encoder speed.
#[derive(Clone, Copy, Debug)]
pub struct Archived {
  pub max_dimension: u32,
  pub quality: u8,
  pub speed: u8,
}

/// A picture as a document of the archive keeps it: upright, brought inside the largest side and written as
/// AVIF, transparency and all. A GIF or a WebP, which may be animated and would lose its animation in a still
/// AVIF, is kept as it came, and so is a picture that cannot be redrawn: none is answered for either.
pub fn archived(bytes: &[u8], archived: Archived) -> Option<Vec<u8>> {
  if matches!(image::guess_format(bytes), Ok(ImageFormat::Gif | ImageFormat::WebP)) {
    return None;
  }

  let picture = decode(bytes, true).ok()?;
  let (width, height) = inside(picture.width(), picture.height(), archived.max_dimension);
  let brought = if (width, height) == (picture.width(), picture.height()) {
    picture
  } else {
    picture.resize_exact(width, height, FilterType::Lanczos3)
  };
  let mut out = Vec::new();
  let encoder =
    image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut out, archived.speed, archived.quality);

  brought.to_rgba8().write_with_encoder(encoder).ok()?;

  Some(out)
}

// The narrowest and the widest copy of a picture drawn small, and how much wider each step may be than the
// one before (`docs/artifact.md`, "A picture drawn small").
const FIRST_STEP: u32 = 16;
const LAST_STEP: u32 = 2048;
// How hard a drawn copy is squeezed where it is written as JPEG.
const DRAWN_QUALITY: u8 = 86;

/// The widths a picture drawn small is copied at, narrowest first, each at most a quarter wider than the one
/// before.
pub fn drawn_steps() -> Vec<u32> {
  let mut steps = Vec::new();
  let mut width = FIRST_STEP;

  while width < LAST_STEP {
    steps.push(width);
    width = width * 5 / 4;
  }

  steps.push(LAST_STEP);
  steps
}

/// The step a width asked for is answered at: the first at or above it, and the widest past the last.
pub fn drawn_step(width: u32) -> u32 {
  drawn_steps().into_iter().find(|step| *step >= width).unwrap_or(LAST_STEP)
}

/// A picture as it is drawn at `width` pixels across, at its own shape: upright, shrunk with a Lanczos kernel
/// and written as JPEG where it is opaque and PNG where it is not, with the type it is written in. None where
/// it is already no wider, may move (a GIF or a WebP) or is no picture this module reads.
pub fn drawn(bytes: &[u8], width: u32) -> Option<(Vec<u8>, &'static str)> {
  if matches!(image::guess_format(bytes), Ok(ImageFormat::Gif | ImageFormat::WebP)) {
    return None;
  }

  let picture = decode(bytes, true).ok()?;

  if picture.width() <= width {
    return None;
  }

  let height = ((u64::from(picture.height()) * u64::from(width) + u64::from(picture.width()) / 2)
    / u64::from(picture.width()))
  .max(1) as u32;
  let shrunk = picture.resize_exact(width, height, FilterType::Lanczos3);
  let mut out = Vec::new();

  if shrunk.to_rgba8().pixels().all(|pixel| pixel.0[3] == u8::MAX) {
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, DRAWN_QUALITY);

    shrunk.to_rgb8().write_with_encoder(encoder).ok()?;

    Some((out, "image/jpeg"))
  } else {
    shrunk.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).ok()?;

    Some((out, "image/png"))
  }
}
