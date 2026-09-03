use std::collections::HashSet;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use syn::{
  GenericParam, Generics, Type,
  visit::{self, Visit},
};

pub struct Names {
  occupied: HashSet<String>,
}

impl Names {
  pub fn new(tokens: TokenStream) -> Self {
    let mut names = Self {
      occupied: HashSet::new(),
    };
    names.collect(tokens);
    names
  }

  pub fn fresh(&mut self, prefix: &str) -> Ident {
    let mut suffix = 0;
    loop {
      let name = format!("{prefix}{suffix}");
      if self.occupied.insert(name.clone()) {
        return Ident::new(&name, Span::call_site());
      }
      suffix += 1;
    }
  }

  fn collect(&mut self, tokens: TokenStream) {
    for token in tokens {
      match token {
        TokenTree::Ident(ident) => {
          self.occupied.insert(self::canonical(&ident));
        }
        TokenTree::Group(group) => self.collect(group.stream()),
        _ => {}
      }
    }
  }
}

pub fn needs_marker(generics: &Generics, remaining: impl Iterator<Item = Type>) -> bool {
  let mut names = Referenced::default();
  for ty in remaining {
    names.visit_type(&ty);
  }
  generics.params.iter().any(|param| match param {
    GenericParam::Type(param) => !names.types.contains(&self::canonical(&param.ident)),
    GenericParam::Lifetime(param) => !names
      .lifetimes
      .contains(&self::canonical(&param.lifetime.ident)),
    GenericParam::Const(_) => false,
  })
}

/// Returns the same key for raw and ordinary spellings of an identifier.
pub fn canonical(ident: &Ident) -> String {
  let name = ident.to_string();
  name.strip_prefix("r#").map(str::to_owned).unwrap_or(name)
}

#[derive(Default)]
struct Referenced {
  types: HashSet<String>,
  lifetimes: HashSet<String>,
}

impl<'ast> Visit<'ast> for Referenced {
  fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
    if let Some(first) = path.path.segments.first() {
      self.types.insert(self::canonical(&first.ident));
    }
    visit::visit_type_path(self, path);
  }

  fn visit_lifetime(&mut self, lifetime: &'ast syn::Lifetime) {
    self.lifetimes.insert(self::canonical(&lifetime.ident));
  }
}
