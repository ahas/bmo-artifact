//! A picture laid on white wherever its transparency would be lost (`docs/artifact.md`, "Where a person reads
//! it"). Carried from picture-ground.test.ts.

use std::io::Cursor;

use bmo_artifact::documents::ground::{is_jpeg, is_transparent, jpeg_on_white};
use image::{ImageFormat, Rgba, RgbaImage};

// A diagram as a search keeps one: a transparent ground, a black stroke across the middle, and a
// half-transparent black pixel beside it as an anti-aliased edge has.
fn diagram() -> Vec<u8> {
  let mut drawn = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));

  for x in 0..8 {
    drawn.put_pixel(x, 4, Rgba([0, 0, 0, 255]));
  }

  drawn.put_pixel(3, 3, Rgba([0, 0, 0, 128]));

  png(&drawn)
}

fn png(drawn: &RgbaImage) -> Vec<u8> {
  let mut bytes = Cursor::new(Vec::new());

  drawn.write_to(&mut bytes, ImageFormat::Png).unwrap();
  bytes.into_inner()
}

fn grey_at(bytes: &[u8], x: u32, y: u32) -> [u8; 3] {
  image::load_from_memory(bytes).unwrap().to_rgb8().get_pixel(x, y).0
}

// What JPEG fills transparency with when nothing is said is black, which is what left a diagram's black
// lines black on black in a room.
#[test]
fn turns_the_transparent_ground_white_keeps_the_stroke_black_and_blends_the_soft_edge() {
  let jpeg = jpeg_on_white(&diagram(), 100).unwrap();

  assert!(is_jpeg(&jpeg));
  assert!(grey_at(&jpeg, 0, 0).iter().all(|value| *value > 245));
  assert!(grey_at(&jpeg, 0, 4).iter().all(|value| *value < 10));
  assert!(grey_at(&jpeg, 3, 3).iter().all(|value| *value > 100 && *value < 160));
}

#[test]
fn is_asked_for_only_where_a_pixel_is_less_than_opaque() {
  let opaque_with_alpha = png(&RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255])));

  assert!(is_transparent(&diagram()).unwrap());
  assert!(!is_transparent(&opaque_with_alpha).unwrap());
  assert!(is_transparent(b"no picture").is_err());
}
