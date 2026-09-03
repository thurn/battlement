use std::collections::BTreeMap;

use quote::ToTokens;
use syn::{
  GenericParam, Generics, Path, Type, TypeParamBound, WherePredicate,
  visit_mut::{self, VisitMut},
};

use crate::names;

/// Qualifies associated types in generic defaults, where Rust requires the trait.
pub fn default_type(ty: &Type, generics: &Generics) -> syn::Result<Type> {
  let mut projection = Projection {
    bounds: BTreeMap::new(),
    error: None,
  };
  for parameter in &generics.params {
    if let GenericParam::Type(parameter) = parameter {
      projection.add(&names::canonical(&parameter.ident), parameter.bounds.iter());
    }
  }
  if let Some(clause) = &generics.where_clause {
    for predicate in &clause.predicates {
      if let WherePredicate::Type(predicate) = predicate {
        if let Type::Path(ty) = &predicate.bounded_ty {
          if let Some(ident) = ty.path.get_ident() {
            projection.add(&names::canonical(ident), predicate.bounds.iter());
          }
        }
      }
    }
  }
  let mut ty = ty.clone();
  projection.visit_type_mut(&mut ty);
  match projection.error {
    Some(error) => Err(error),
    None => Ok(ty),
  }
}

struct Projection {
  bounds: BTreeMap<String, Vec<Path>>,
  error: Option<syn::Error>,
}

impl Projection {
  fn add<'a>(&mut self, name: &str, bounds: impl Iterator<Item = &'a TypeParamBound>) {
    let paths = self.bounds.entry(name.to_owned()).or_default();
    for bound in bounds {
      if let TypeParamBound::Trait(bound) = bound {
        let tokens = bound.path.to_token_stream().to_string();
        if !paths
          .iter()
          .any(|path| path.to_token_stream().to_string() == tokens)
        {
          paths.push(bound.path.clone());
        }
      }
    }
  }
}

impl VisitMut for Projection {
  fn visit_type_mut(&mut self, ty: &mut Type) {
    visit_mut::visit_type_mut(self, ty);
    let Type::Path(path) = ty else {
      return;
    };
    if path.qself.is_some() || path.path.segments.len() < 2 {
      return;
    }
    let parameter = &path.path.segments[0].ident;
    let Some(bounds) = self.bounds.get(&names::canonical(parameter)) else {
      return;
    };
    let [bound] = bounds.as_slice() else {
      self.error = Some(syn::Error::new_spanned(
        &*ty,
        "spell this required associated type as `<T as Trait>::Associated` so its builder slot has an unambiguous default",
      ));
      return;
    };
    let tail = path.path.segments.iter().skip(1);
    *ty = syn::parse_quote!(<#parameter as #bound>::#(#tail)::*);
  }
}
