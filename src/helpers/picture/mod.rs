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

// The size a picture is brought inside a square of `limit` pixels at, never larger than it stands.
fn inside(width: u32, height: u32, limit: u32) -> (u32, u32) {
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
