use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use syn::{
  Attribute, Error, Expr, Field, Fields, ItemStruct, Path, Type,
  parse::Parser,
  punctuated::Punctuated,
  spanned::Spanned,
  visit::{self, Visit},
};

use crate::{conversion::Conversion, names::Names};

pub struct Input {
  pub item: ItemStruct,
  pub fields: Vec<Property>,
  pub support: Path,
  pub names: Names,
}

pub struct Property {
  pub field: Field,
  pub required: bool,
  pub default: Option<Expr>,
  pub conversion: Conversion,
  pub clear: Option<Ident>,
  pub forward: Option<Ident>,
  pub slot: Option<Ident>,
}

pub fn parse(arguments: TokenStream, tokens: TokenStream) -> syn::Result<Input> {
  let mut support = None;
  syn::meta::parser(|meta| {
    if !meta.path.is_ident("support") {
      return Err(meta.error("expected `support = ::module`"));
    }
    if support.is_some() {
      return Err(meta.error("duplicate builder support option"));
    }
    support = Some(meta.value()?.parse()?);
    Ok(())
  })
  .parse2(arguments)?;
  let mut item: ItemStruct = syn::parse2(tokens.clone()).map_err(|error| {
    Error::new(
      error.span(),
      "#[builder] requires a named-field or unit struct",
    )
  })?;
  if matches!(item.fields, Fields::Unnamed(_)) {
    return Err(Error::new(
      item.fields.span(),
      "tuple structs do not support named property setters",
    ));
  }
  let mut unsupported = Unsupported {
    name: &item.ident,
    error: None,
  };
  unsupported.visit_generics(&item.generics);
  for field in &item.fields {
    unsupported.visit_type(&field.ty);
  }
  if let Some(error) = unsupported.error {
    return Err(error);
  }
  let mut names = Names::new(tokens);
  let mut fields = Vec::new();
  let mut methods = HashSet::from(["new".to_owned()]);
  for field in &mut item.fields {
    let mut required = false;
    let mut into = false;
    let mut default = None;
    let mut keys = HashSet::new();
    for attr in &field.attrs {
      if attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr") {
        return Err(Error::new_spanned(
          attr,
          "conditionally compiled builder fields are unsupported",
        ));
      }
      if !attr.path().is_ident("builder") {
        continue;
      }
      attr.parse_nested_meta(|meta| {
        let key = meta
          .path
          .get_ident()
          .map(ToString::to_string)
          .unwrap_or_default();
        if !keys.insert(key.clone()) {
          return Err(meta.error("duplicate builder field option"));
        }
        match key.as_str() {
          "required" => required = true,
          "into" => into = true,
          "default" => default = Some(meta.value()?.parse()?),
          _ => return Err(meta.error("expected `required`, `into`, or `default = expression`")),
        }
        Ok(())
      })?;
    }
    if required && default.is_some() {
      return Err(Error::new_spanned(
        &*field,
        "a required property cannot have a default",
      ));
    }
    let mut conversion = Conversion::classify(&field.ty);
    if into {
      if conversion.is_callback() {
        return Err(Error::new_spanned(
          &field.ty,
          "callback properties already convert closures; remove `into`",
        ));
      }
      conversion = Conversion::Into;
    }
    let ident = field.ident.as_ref().expect("named field");
    let name = ident.to_string().trim_start_matches("r#").to_owned();
    if !methods.insert(name.clone()) {
      return Err(Error::new_spanned(
        ident,
        "property conflicts with another generated builder method",
      ));
    }
    let clear = if conversion.can_clear() && !required {
      let clear = format!("clear_{name}");
      if !methods.insert(clear.clone()) {
        return Err(Error::new_spanned(
          ident,
          "callback clearing method conflicts with a property",
        ));
      }
      Some(Ident::new(&clear, ident.span()))
    } else {
      None
    };
    let forward = conversion
      .is_optional_callback()
      .then(|| {
        let forward = format!("{name}_optional");
        if !methods.insert(forward.clone()) {
          return Err(Error::new_spanned(
            ident,
            "optional callback forwarding method conflicts with a property",
          ));
        }
        Ok(Ident::new(&forward, ident.span()))
      })
      .transpose()?;
    field.attrs.retain(|attr| !attr.path().is_ident("builder"));
    fields.push(Property {
      field: field.clone(),
      required,
      default,
      conversion,
      clear,
      forward,
      slot: required.then(|| names.fresh("__BuilderField")),
    });
  }
  Ok(Input {
    item,
    fields,
    names,
    support: support
      .unwrap_or_else(|| syn::parse_quote!(::battlement_reactant::prelude::__builder_support)),
  })
}

pub fn conditions(attributes: &[Attribute]) -> syn::Result<Vec<Attribute>> {
  attributes
    .iter()
    .filter_map(|attribute| match self::condition(&attribute.meta) {
      Ok(Some(meta)) => Some(Ok(syn::parse_quote!(#[#meta]))),
      Ok(None) => None,
      Err(error) => Some(Err(error)),
    })
    .collect()
}

fn condition(meta: &syn::Meta) -> syn::Result<Option<syn::Meta>> {
  if meta.path().is_ident("cfg") {
    return Ok(Some(meta.clone()));
  }
  if !meta.path().is_ident("cfg_attr") {
    return Ok(None);
  }
  let arguments = meta
    .require_list()?
    .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)?;
  let mut arguments = arguments.iter();
  let predicate = arguments
    .next()
    .ok_or_else(|| Error::new_spanned(meta, "missing cfg_attr condition"))?;
  let nested = arguments
    .filter_map(|meta| self::condition(meta).transpose())
    .collect::<syn::Result<Vec<_>>>()?;
  if nested.is_empty() {
    Ok(None)
  } else {
    Ok(Some(syn::parse_quote!(cfg_attr(#predicate, #(#nested),*))))
  }
}

struct Unsupported<'a> {
  name: &'a Ident,
  error: Option<Error>,
}

impl<'ast> Visit<'ast> for Unsupported<'_> {
  fn visit_path(&mut self, path: &'ast Path) {
    if let Some(segment) = path.segments.last() {
      if segment.ident == "Self" || segment.ident == *self.name {
        self.error = Some(Error::new_spanned(
          path,
          "self-dependent and recursive builder declarations are unsupported",
        ));
      }
    }
    if path
      .segments
      .first()
      .is_some_and(|segment| segment.ident == "Self")
    {
      self.error = Some(Error::new_spanned(
        path,
        "Self-dependent builder bounds and fields are unsupported",
      ));
    }
    visit::visit_path(self, path);
  }

  fn visit_type(&mut self, ty: &'ast Type) {
    visit::visit_type(self, ty);
  }
}
