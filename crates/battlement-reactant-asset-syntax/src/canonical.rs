use sha2::{Digest, Sha256};

pub(crate) fn identity(bytes: &[u8]) -> [u8; 32] {
  Sha256::digest(bytes).into()
}

pub(crate) fn number(bytes: &mut Vec<u8>, value: f64) {
  bytes.extend(
    if value == 0.0 { 0.0 } else { value }
      .to_bits()
      .to_be_bytes(),
  );
}

pub(crate) fn string(bytes: &mut Vec<u8>, value: &str) {
  self::blob(bytes, value.as_bytes());
}

pub(crate) fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
  self::length(bytes, value.len());
  bytes.extend(value);
}

fn length(bytes: &mut Vec<u8>, value: usize) {
  bytes.extend(
    u32::try_from(value)
      .expect("canonical collection length overflow")
      .to_be_bytes(),
  );
}
