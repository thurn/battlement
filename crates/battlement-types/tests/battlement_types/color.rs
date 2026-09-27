use battlement_types::Color;

#[test]
fn creates_colors_from_authoring_friendly_values() {
  assert_eq!(Color::TRANSPARENT, Color::rgba(0.0, 0.0, 0.0, 0.0));
  assert_eq!(Color::hex(0x2b4a7b), Color::rgb8(43, 74, 123));
  assert_eq!(Color::hex_rgba(0x2b4a7b40), Color::rgba8(43, 74, 123, 64));
  assert_eq!(
    Color::rgba8(43, 74, 123, 64),
    Color::rgba(43.0 / 255.0, 74.0 / 255.0, 123.0 / 255.0, 64.0 / 255.0),
  );
  assert_eq!(
    Color::hex(0x2b4a7b).with_alpha(0.25),
    Color::rgba(43.0 / 255.0, 74.0 / 255.0, 123.0 / 255.0, 0.25),
  );
}

#[test]
#[should_panic(expected = "an RGB hex color must fit in 24 bits")]
fn rejects_oversized_rgb_hex_values() {
  let _ = Color::hex(0x12_345678);
}
