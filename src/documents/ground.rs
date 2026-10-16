//! The white a picture is laid on wherever its transparency would be lost, and the picture a share link's
//! preview card shows (`docs/archive.md`, "The share page"). JPEG has no transparency, and what fills it when
//! nothing is said is black; a chat app or a dark screen shows it on its own dark ground. The diagrams a
//! search keeps are black lines and words on a transparent ground, drawn for paper, so on either they
//! vanish. A picture with no transparency covers the white wholly and never shows it.

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, Rgb, RgbImage};

use crate::helpers::picture::{inside, upright};

/// The ground a picture is laid on.
pub const PICTURE_GROUND: Rgb<u8> = Rgb([255, 255, 255]);

/// The long side of a share link's card: what the widest preview card is drawn at.
pub const CARD_EDGE: u32 = 1200;
/// What a card's picture is written as, and at what quality.
pub const CARD_TYPE: &str = "image/jpeg";
pub const CARD_QUALITY: u8 = 82;

/// Whether bytes are a JPEG, read off their first bytes. A JPEG has no transparency to lay on anything, so it
/// is the one picture that is never decoded to ask.
pub fn is_jpeg(bytes: &[u8]) -> bool {
  bytes.starts_with(&[0xff, 0xd8, 0xff])
}

fn decoded(bytes: &[u8]) -> Result<DynamicImage, String> {
  upright(bytes).map_err(|why| why.0)
}

/// Whether any pixel of the picture is less than opaque. A PNG may carry an alpha channel every pixel of
/// which is opaque, and that picture has nothing to lay on white. Refused for bytes that are no picture.
pub fn is_transparent(bytes: &[u8]) -> Result<bool, String> {
  let picture = decoded(bytes)?;

  Ok(picture.color().has_alpha() && picture.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX))
}

// A picture laid on white and written as JPEG at `quality`. Each edge pixel is composited by its own alpha,
// so an anti-aliased line keeps its soft edge against the white.
fn laid_on_white(picture: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
  let picture = picture.to_rgba8();
  let ground = PICTURE_GROUND.0;
  let laid = RgbImage::from_fn(picture.width(), picture.height(), |x, y| {
    let [red, green, blue, alpha] = picture.get_pixel(x, y).0;
    let over = |value: u8, under: u8| {
      ((u32::from(value) * u32::from(alpha) + u32::from(under) * (255 - u32::from(alpha)) + 127) / 255)
        as u8
    };

    Rgb([over(red, ground[0]), over(green, ground[1]), over(blue, ground[2])])
  });
  let mut written = Vec::new();

  JpegEncoder::new_with_quality(&mut written, quality)
    .encode_image(&laid)
    .map_err(|why| why.to_string())?;

  Ok(written)
}

/// The picture upright, laid on white and written as JPEG at `quality`. Refused for bytes that are no
/// picture.
pub fn jpeg_on_white(bytes: &[u8], quality: u8) -> Result<Vec<u8>, String> {
  laid_on_white(&decoded(bytes)?, quality)
}

/// A document's picture as a link to it shows it in a chat app's preview card: its first frame, upright,
/// fitted inside the card's edge and never enlarged, laid on white and written as JPEG, the one format every
/// crawler drawing those cards reads. None for bytes that are no picture.
pub fn card_picture(bytes: &[u8]) -> Option<Vec<u8>> {
  let picture = decoded(bytes).ok()?;
  let (width, height) = inside(picture.width(), picture.height(), CARD_EDGE);
  let fitted = if (width, height) == (picture.width(), picture.height()) {
    picture
  } else {
    picture.resize_exact(width, height, FilterType::Lanczos3)
  };

  laid_on_white(&fitted, CARD_QUALITY).ok()
}
