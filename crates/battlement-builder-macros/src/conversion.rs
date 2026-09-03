use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{GenericArgument, Path, PathArguments, Type, TypeParamBound};

pub enum Conversion {
  Exact,
  Into,
  Option(Type),
  Callback {
    bounds: Vec<TypeParamBound>,
    optional: bool,
  },
  Event {
    payload: Type,
    optional: bool,
  },
}

impl Conversion {
  pub fn classify(ty: &Type) -> Self {
    let inner = self::argument(
      ty,
      &["Option", "std::option::Option", "core::option::Option"],
    );
    let target = inner.as_ref().unwrap_or(ty);
    let optional = inner.is_some();
    if let Some(Type::TraitObject(object)) =
      self::argument(target, &["Rc", "std::rc::Rc", "alloc::rc::Rc"])
    {
      if object.bounds.iter().any(self::is_fn) {
        let mut bounds: Vec<_> = object.bounds.into_iter().collect();
        if !bounds
          .iter()
          .any(|bound| matches!(bound, TypeParamBound::Lifetime(_)))
        {
          bounds.push(syn::parse_quote!('static));
        }
        return Self::Callback { bounds, optional };
      }
    }
    if let Some(payload) = self::argument(
      target,
      &[
        "EventCallback",
        "battlement_reactant::prelude::EventCallback",
        "battlement_reactant::callback::Callback",
      ],
    ) {
      return Self::Event { payload, optional };
    }
    if let Some(inner) = inner {
      return Self::Option(inner);
    }
    if self::matches_path(
      ty,
      &["String", "std::string::String", "alloc::string::String"],
    ) {
      Self::Into
    } else {
      Self::Exact
    }
  }

  pub fn is_callback(&self) -> bool {
    matches!(self, Self::Callback { .. } | Self::Event { .. })
  }

  pub fn can_clear(&self) -> bool {
    matches!(
      self,
      Self::Callback { optional: true, .. } | Self::Event { optional: true, .. }
    )
  }

  pub fn is_optional_callback(&self) -> bool {
    matches!(self, Self::Callback { optional: true, .. })
  }

  pub fn input(&self, ty: &Type, support: &Path, signature: &Ident) -> TokenStream {
    match self {
      Self::Exact => quote!(#ty),
      Self::Into => quote!(impl ::core::convert::Into<#ty>),
      Self::Option(inner) => quote!(impl #support::IntoOption<#inner>),
      Self::Callback { bounds, .. } => quote!(impl #(#bounds)+*),
      Self::Event { payload, .. } => quote!(impl #support::IntoEventCallback<#payload, #signature>),
    }
  }

  pub fn value(&self, support: &Path, value: &Ident) -> TokenStream {
    match self {
      Self::Exact => quote!(#value),
      Self::Into => quote!(::core::convert::Into::into(#value)),
      Self::Option(_) => quote!(#support::IntoOption::into_option(#value)),
      Self::Callback { optional, .. } => self::wrap(quote!(::std::rc::Rc::new(#value)), *optional),
      Self::Event { optional, .. } => self::wrap(
        quote!(#support::IntoEventCallback::into_callback(#value)),
        *optional,
      ),
    }
  }
}

fn wrap(value: TokenStream, optional: bool) -> TokenStream {
  if optional {
    quote!(::core::option::Option::Some(#value))
  } else {
    value
  }
}

fn is_fn(bound: &TypeParamBound) -> bool {
  let TypeParamBound::Trait(bound) = bound else {
    return false;
  };
  let name = bound
    .path
    .segments
    .iter()
    .map(|part| part.ident.to_string())
    .collect::<Vec<_>>()
    .join("::");
  matches!(name.as_str(), "Fn" | "std::ops::Fn" | "core::ops::Fn")
}

fn matches_path(ty: &Type, names: &[&str]) -> bool {
  let Type::Path(ty) = ty else {
    return false;
  };
  ty.qself.is_none()
    && names.contains(
      &ty
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
        .as_str(),
    )
}

fn argument(ty: &Type, names: &[&str]) -> Option<Type> {
  if !self::matches_path(ty, names) {
    return None;
  }
  let Type::Path(ty) = ty else {
    return None;
  };
  let PathArguments::AngleBracketed(arguments) = &ty.path.segments.last()?.arguments else {
    return None;
  };
  if arguments.args.len() != 1 {
    return None;
  }
  match arguments.args.first()? {
    GenericArgument::Type(ty) => Some(ty.clone()),
    _ => None,
  }
}
