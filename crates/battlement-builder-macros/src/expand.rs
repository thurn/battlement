use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Attribute, Fields, GenericParam, Generics, Type, spanned::Spanned};

use crate::{
  conversion::Conversion,
  input::{self, Input, Property},
  names, projection,
};

pub fn expand(mut input: Input) -> syn::Result<TokenStream> {
  let conditions = input::conditions(&input.item.attrs)?;
  let original_generics = input.item.generics.clone();
  let marker = if !input.fields.iter().any(|field| field.required) {
    None
  } else {
    names::needs_marker(
      &original_generics,
      input
        .fields
        .iter()
        .filter(|field| !field.required)
        .map(|field| field.field.ty.clone()),
    )
    .then(|| input.names.fresh("__builder_marker"))
  };
  for (field, property) in input.item.fields.iter_mut().zip(&input.fields) {
    if let Some(slot) = &property.slot {
      let ty = projection::default_type(&field.ty, &original_generics)?;
      input
        .item
        .generics
        .params
        .push(syn::parse_quote!(#slot = #ty));
      field.ty = syn::parse_quote!(#slot);
    }
  }
  if let Some(marker) = &marker {
    let required = input
      .fields
      .iter()
      .filter(|field| field.required)
      .map(|field| &field.field.ty);
    let Fields::Named(fields) = &mut input.item.fields else {
      unreachable!()
    };
    fields.named.push(syn::parse_quote! {
      #marker: ::core::marker::PhantomData<fn() -> (#(*const #required,)*)>
    });
  }
  let constructor = self::constructor(&input, &original_generics, marker.as_ref(), &conditions);
  let mut implementations = Vec::new();
  let mut optional = Vec::new();
  for index in 0..input.fields.len() {
    let signature = input.names.fresh("__BuilderSignature");
    let value = input.names.fresh("__builder_value");
    let method = self::setter(
      &input,
      index,
      &original_generics,
      marker.as_ref(),
      &signature,
      &value,
      false,
    );
    let property = &input.fields[index];
    let forward = property.forward.as_ref().map(|_| {
      self::setter(
        &input,
        index,
        &original_generics,
        marker.as_ref(),
        &signature,
        &value,
        true,
      )
    });
    if let Some(slot) = &property.slot {
      let mut generics = input.item.generics.clone();
      generics.params = generics
        .params
        .into_iter()
        .filter(|param| !matches!(param, GenericParam::Type(param) if param.ident == *slot))
        .collect();
      let (params, _, bounds) = generics.split_for_impl();
      let receiver = self::state_type(&input, &original_generics, Some((index, false)), false);
      implementations
        .push(quote!(#(#conditions)* impl #params #receiver #bounds { #method #forward }));
    } else {
      optional.push(method);
      optional.extend(forward);
      if let Some(clear) = &property.clear {
        let ident = &property.field.ident;
        let visibility = &input.item.vis;
        let docs = self::docs(property);
        let description = format!(
          "Removes the callback supplied through [`Self::{}`].",
          ident.as_ref().unwrap()
        );
        optional.push(quote! {
          #[doc = #description]
          #[doc = ""]
          #(#docs)*
          #[must_use]
          #visibility fn #clear(mut self) -> Self {
            self.#ident = ::core::option::Option::None;
            self
          }
        });
      }
    }
  }
  let (params, ty, bounds) = input.item.generics.split_for_impl();
  let name = &input.item.ident;
  let optional_impl = if optional.is_empty() {
    quote!()
  } else {
    quote!(#(#conditions)* impl #params #name #ty #bounds { #(#optional)* })
  };
  let item = &input.item;
  Ok(quote!(#item #constructor #(#implementations)* #optional_impl))
}

fn constructor(
  input: &Input,
  original: &Generics,
  marker: Option<&Ident>,
  conditions: &[Attribute],
) -> TokenStream {
  let name = &input.item.ident;
  let visibility = &input.item.vis;
  let support = &input.support;
  let mut generics = original.clone();
  for property in &input.fields {
    if !property.required && property.default.is_none() {
      let ty = &property.field.ty;
      generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote!(#ty: ::core::default::Default));
    }
  }
  // Keep constructor-only bounds off the struct and its setters.
  let (params, ty, bounds) = original.split_for_impl();
  let constructor_bounds = &generics.where_clause;
  let result = self::state_type(input, original, None, true);
  let assignments = input.fields.iter().map(|property| {
    let ident = &property.field.ident;
    let ty = &property.field.ty;
    let value = if property.required {
      quote!(#support::Missing::<#ty>::new())
    } else if let Some(default) = &property.default {
      quote!(#default)
    } else {
      quote_spanned!(ty.span()=> <#ty as ::core::default::Default>::default())
    };
    quote!(#ident: #value)
  });
  let marker = marker.map(|marker| quote!(#marker: ::core::marker::PhantomData,));
  let body = if matches!(input.item.fields, Fields::Unit) {
    quote!(#name)
  } else {
    quote!(#name { #(#assignments,)* #marker })
  };
  quote! {
    #(#conditions)*
    impl #params #name #ty #bounds {
      /// Creates a value with default optional props and unfilled required props.
      #[must_use]
      #visibility fn new() -> #result #constructor_bounds { #body }
    }
  }
}

fn setter(
  input: &Input,
  index: usize,
  original: &Generics,
  marker: Option<&Ident>,
  signature: &Ident,
  value: &Ident,
  forwarding: bool,
) -> TokenStream {
  let property = &input.fields[index];
  let field = property.field.ident.as_ref().expect("named field");
  let ident = if forwarding {
    property
      .forward
      .as_ref()
      .expect("optional callback forwarder")
  } else {
    field
  };
  let ty = &property.field.ty;
  let visibility = &input.item.vis;
  let (argument, converted) = if forwarding {
    (quote!(#ty), quote!(#value))
  } else {
    (
      property.conversion.input(ty, &input.support, signature),
      property.conversion.value(&input.support, value),
    )
  };
  let docs = if forwarding {
    Vec::new()
  } else {
    self::docs(property)
  };
  let forwarding_doc = forwarding.then(|| {
    let description =
      format!("Forwards a stored optional callback without rewrapping [`Self::{field}`].");
    quote!(#[doc = #description])
  });
  let forwarding_lints = forwarding.then(|| quote!(#[allow(clippy::type_complexity)]));
  let parameters = if matches!(property.conversion, Conversion::Event { .. }) {
    quote!(<#signature: 'static>)
  } else {
    quote!()
  };
  if !property.required {
    return quote! {
      #forwarding_doc
      #forwarding_lints
      #(#docs)*
      #[must_use]
      #visibility fn #ident #parameters(mut self, #value: #argument) -> Self {
        self.#field = #converted;
        self
      }
    };
  }
  let result = self::state_type(input, original, Some((index, true)), false);
  let name = &input.item.ident;
  let assignments = input.fields.iter().enumerate().map(|(other_index, other)| {
    let field = &other.field.ident;
    if other_index == index {
      quote!(#field: #value)
    } else {
      quote!(#field: self.#field)
    }
  });
  let marker = marker.map(|marker| quote!(#marker: self.#marker,));
  quote! {
    #forwarding_doc
    #forwarding_lints
    #(#docs)*
    #[must_use]
    #visibility fn #ident #parameters(self, #value: #argument) -> #result {
      let #value: #ty = #converted;
      #name { #(#assignments,)* #marker }
    }
  }
}

fn state_type(
  input: &Input,
  original: &Generics,
  change: Option<(usize, bool)>,
  missing: bool,
) -> TokenStream {
  let name = &input.item.ident;
  let support = &input.support;
  let mut arguments: Vec<TokenStream> = original
    .params
    .iter()
    .map(|param| match param {
      GenericParam::Lifetime(param) => {
        let lifetime = &param.lifetime;
        quote!(#lifetime)
      }
      GenericParam::Type(param) => {
        let ident = &param.ident;
        quote!(#ident)
      }
      GenericParam::Const(param) => {
        let ident = &param.ident;
        quote!(#ident)
      }
    })
    .collect();
  for (index, property) in input.fields.iter().enumerate() {
    let Some(slot) = &property.slot else {
      continue;
    };
    let ty: &Type = &property.field.ty;
    arguments.push(if missing || change == Some((index, false)) {
      quote!(#support::Missing<#ty>)
    } else if change == Some((index, true)) {
      quote!(#ty)
    } else {
      quote!(#slot)
    });
  }
  if arguments.is_empty() {
    quote!(#name)
  } else {
    quote!(#name<#(#arguments),*>)
  }
}

fn docs(property: &Property) -> Vec<Attribute> {
  property
    .field
    .attrs
    .iter()
    .filter(|attr| attr.path().is_ident("doc"))
    .cloned()
    .collect()
}
