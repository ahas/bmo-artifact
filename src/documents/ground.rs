//! The white a picture is laid on wherever its transparency would be lost. JPEG has no transparency, and what
//! fills it when nothing is said is black; a chat app or a dark screen shows it on its own dark ground. The
//! diagrams a search keeps are black lines and words on a transparent ground, drawn for paper, so on either
//! they vanish. A picture with no transparency covers the white wholly and never shows it.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageDecoder, ImageReader, Rgb, RgbImage};

/// The ground a picture is laid on.
pub const PICTURE_GROUND: Rgb<u8> = Rgb([255, 255, 255]);

/// Whether bytes are a JPEG, read off their first bytes. A JPEG has no transparency to lay on anything, so it
/// is the one picture that is never decoded to ask.
pub fn is_jpeg(bytes: &[u8]) -> bool {
  bytes.starts_with(&[0xff, 0xd8, 0xff])
}

// The picture bytes hold, turned upright as they ask.
fn upright(bytes: &[u8]) -> Result<DynamicImage, String> {
  let reader = ImageReader::new(Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|why| why.to_string())?;
  let mut decoder = reader.into_decoder().map_err(|why| why.to_string())?;
  let orientation = decoder.orientation().map_err(|why| why.to_string())?;
  let mut picture = DynamicImage::from_decoder(decoder).map_err(|why| why.to_string())?;

  picture.apply_orientation(orientation);

  Ok(picture)
}

/// Whether any pixel of the picture is less than opaque. A PNG may carry an alpha channel every pixel of
/// which is opaque, and that picture has nothing to lay on white. Refused for bytes that are no picture.
pub fn is_transparent(bytes: &[u8]) -> Result<bool, String> {
  let picture = upright(bytes)?;

  Ok(picture.color().has_alpha() && picture.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX))
}

/// The picture upright, laid on white and written as JPEG at `quality`. Each edge pixel is composited by its
/// own alpha, so an anti-aliased line keeps its soft edge against the white. Refused for bytes that are no
/// picture.
pub fn jpeg_on_white(bytes: &[u8], quality: u8) -> Result<Vec<u8>, String> {
  let picture = upright(bytes)?.to_rgba8();
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
