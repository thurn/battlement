use crate::canonical;

use super::{CalcNode, Calculation, Unit, Value};

pub(super) fn value(value: &Value, bytes: &mut Vec<u8>) {
  match value {
    Value::Scalar(value) => {
      bytes.push(1);
      bytes.push(self::unit_tag(value.unit));
      canonical::number(bytes, value.value);
    }
    Value::Color(channels) => {
      bytes.push(2);
      for channel in channels {
        bytes.extend(channel.to_bits().to_be_bytes());
      }
    }
    Value::String(value) => {
      bytes.push(3);
      canonical::string(bytes, value);
    }
    Value::Keyword(value) => {
      bytes.push(4);
      canonical::string(bytes, value);
    }
    Value::Function(name, value) => {
      bytes.push(5);
      canonical::string(bytes, name);
      self::value(value, bytes);
    }
    Value::Space(values) => self::list(6, values, bytes),
    Value::Comma(values) => self::list(7, values, bytes),
    Value::Calculation(value) => {
      bytes.push(8);
      self::calculation(value, bytes);
    }
  }
}

fn calculation(value: &Calculation, bytes: &mut Vec<u8>) {
  match &value.node {
    CalcNode::Scalar(value) => self::value(&Value::Scalar(*value), bytes),
    CalcNode::Add(left, right) => self::binary(1, left, right, bytes),
    CalcNode::Subtract(left, right) => self::binary(2, left, right, bytes),
    CalcNode::Multiply(left, right) => self::binary(3, left, right, bytes),
    CalcNode::Divide(left, right) => self::binary(4, left, right, bytes),
    CalcNode::Min(values) => self::calculation_list(5, values, bytes),
    CalcNode::Max(values) => self::calculation_list(6, values, bytes),
    CalcNode::Clamp(values) => self::calculation_list(7, values, bytes),
    CalcNode::Group(value) => {
      bytes.push(8);
      self::calculation(value, bytes);
    }
  }
}

fn binary(tag: u8, left: &Calculation, right: &Calculation, bytes: &mut Vec<u8>) {
  bytes.push(tag);
  self::calculation(left, bytes);
  self::calculation(right, bytes);
}

fn calculation_list(tag: u8, values: &[Calculation], bytes: &mut Vec<u8>) {
  bytes.push(tag);
  bytes.push(u8::try_from(values.len()).expect("calculation list length overflow"));
  for value in values {
    self::calculation(value, bytes);
  }
}

fn list(tag: u8, values: &[Value], bytes: &mut Vec<u8>) {
  bytes.push(tag);
  bytes.extend(
    u32::try_from(values.len())
      .expect("CSS list length overflow")
      .to_be_bytes(),
  );
  for value in values {
    self::value(value, bytes);
  }
}

fn unit_tag(unit: Unit) -> u8 {
  match unit {
    Unit::Number => 1,
    Unit::Percent => 2,
    Unit::Px => 3,
    Unit::Em => 4,
    Unit::Rem => 5,
    Unit::Vw => 6,
    Unit::Vh => 7,
    Unit::Vmin => 8,
    Unit::Vmax => 9,
    Unit::Deg => 10,
    Unit::Grad => 11,
    Unit::Rad => 12,
    Unit::Turn => 13,
  }
}
