//! Typed shader declarations and renderer-local values.

use std::marker::PhantomData;

use crate::{Color, MaterialAddress, PreparedAsset};

/// Supported continuous shader property types.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MaterialParameterKind {
  /// Scalar float or shader range.
  Float,
  /// Linear RGBA color.
  Color,
  /// Four-dimensional numeric vector.
  Vector,
}

/// One required named property in a prepared material's shader.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MaterialParameterDeclaration {
  /// Exact shader property name.
  pub name: String,
  /// Required shader property type.
  pub kind: MaterialParameterKind,
}

/// A shader property whose accepted value is checked by Rust.
#[derive(Clone, Copy, Debug)]
pub struct MaterialParameter<T> {
  name: &'static str,
  value: PhantomData<fn() -> T>,
}

/// An explicitly four-dimensional shader vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialVector(
  /// X, Y, Z, W components.
  pub [f64; 4],
);

/// A concrete renderer-local shader value, also usable by property drivers.
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialValue {
  /// Scalar float or shader range.
  Float(f64),
  /// Linear RGBA color.
  Color(Color),
  /// Four-dimensional numeric vector.
  Vector(MaterialVector),
}

/// Converts supported Rust values into typed material values.
pub trait IntoMaterialValue: private::Sealed {
  /// Converts to the corresponding protocol value.
  fn into_material_value(self) -> MaterialValue;
}

/// A named renderer-local value.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialParameterValue {
  /// Exact shader property name.
  pub name: String,
  /// Renderer-local value.
  pub value: MaterialValue,
}

/// One prepared material and its overrides for a renderer slot.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialInstance {
  /// Prepared shared material.
  pub address: MaterialAddress,
  /// Authored renderer material slot.
  pub slot: u32,
  /// Named overrides, unique by name.
  pub parameters: Vec<MaterialParameterValue>,
}

impl<T: IntoMaterialValue> MaterialParameter<T> {
  /// Declares a shader property with a Rust value type.
  pub const fn new(name: &'static str) -> Self {
    Self {
      name,
      value: PhantomData,
    }
  }
  /// Creates a named value from this declaration.
  pub fn value(self, value: T) -> MaterialParameterValue {
    assert!(
      !self.name.is_empty(),
      "material parameter name cannot be empty"
    );
    MaterialParameterValue {
      name: self.name.to_owned(),
      value: value.into_material_value(),
    }
  }
}

impl MaterialInstance {
  /// Uses slot zero of a prepared material without overrides.
  pub fn new(address: impl Into<MaterialAddress>) -> Self {
    Self {
      address: address.into(),
      slot: 0,
      parameters: Vec::new(),
    }
  }
  /// Selects an authored renderer material slot; the default is zero.
  pub fn slot(mut self, slot: u32) -> Self {
    self.slot = slot;
    self
  }
  /// Adds one typed property; repeated names are developer errors.
  pub fn parameter<T: IntoMaterialValue>(
    mut self,
    parameter: MaterialParameter<T>,
    value: T,
  ) -> Self {
    let value = parameter.value(value);
    assert!(
      !self.parameters.iter().any(|p| p.name == value.name),
      "duplicate material parameter"
    );
    self.parameters.push(value);
    self
  }
  /// Whether this prepared asset declares every required override with its exact type.
  pub fn is_declared_by(&self, asset: &PreparedAsset) -> bool {
    match asset {
      PreparedAsset::Material(address) => *address == self.address && self.parameters.is_empty(),
      PreparedAsset::MaterialParameters {
        address,
        parameters,
      } => {
        *address == self.address
          && self.parameters.iter().all(|value| {
            parameters.iter().any(|declaration| {
              declaration.name == value.name && declaration.kind == value.value.kind()
            })
          })
      }
      _ => false,
    }
  }

  /// Required shader metadata, independent of changing instance values.
  pub fn declarations(&self) -> Vec<MaterialParameterDeclaration> {
    self
      .parameters
      .iter()
      .map(|p| MaterialParameterDeclaration {
        name: p.name.clone(),
        kind: p.value.kind(),
      })
      .collect()
  }
}

impl MaterialValue {
  /// Returns whether every component fits a finite native shader number.
  pub fn is_finite(&self) -> bool {
    let components = match self {
      Self::Float(value) => [*value, 0.0, 0.0, 0.0],
      Self::Color(value) => [value.r, value.g, value.b, value.a],
      Self::Vector(value) => value.0,
    };
    components
      .iter()
      .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX))
  }

  /// Returns the shader property type.
  pub fn kind(&self) -> MaterialParameterKind {
    match self {
      Self::Float(_) => MaterialParameterKind::Float,
      Self::Color(_) => MaterialParameterKind::Color,
      Self::Vector(_) => MaterialParameterKind::Vector,
    }
  }
}
impl IntoMaterialValue for f64 {
  fn into_material_value(self) -> MaterialValue {
    MaterialValue::Float(self)
  }
}
impl IntoMaterialValue for Color {
  fn into_material_value(self) -> MaterialValue {
    MaterialValue::Color(self)
  }
}
impl IntoMaterialValue for MaterialVector {
  fn into_material_value(self) -> MaterialValue {
    MaterialValue::Vector(self)
  }
}
mod private {
  pub trait Sealed {}
  impl Sealed for f64 {}
  impl Sealed for crate::Color {}
  impl Sealed for crate::MaterialVector {}
}
