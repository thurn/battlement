//! Procedural macros for Reactant generated assets.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

/// Declares one generated Reactant asset handle and linked registration.
#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
  match battlement_reactant_asset_syntax::parse(&input.to_string()) {
    Ok(request) => self::expand(request).into(),
    Err(error) => {
      let message = error.to_string();
      quote!(compile_error!(#message);).into()
    }
  }
}

/// Declares assets that share one common recipe with per-asset overrides.
#[proc_macro]
pub fn generate_family(input: TokenStream) -> TokenStream {
  let result =
    battlement_reactant_asset_syntax::expand_family(&input.to_string()).and_then(|sources| {
      sources
        .into_iter()
        .map(|source| battlement_reactant_asset_syntax::parse(&source))
        .collect::<Result<Vec<_>, _>>()
    });
  match result {
    Ok(requests) => requests
      .into_iter()
      .map(self::expand)
      .collect::<TokenStream2>()
      .into(),
    Err(error) => {
      let message = error.to_string();
      quote!(compile_error!(#message);).into()
    }
  }
}

fn expand(request: battlement_reactant_asset_syntax::AssetRequest) -> TokenStream2 {
  use battlement_reactant_asset_syntax::DeclarationKind;

  let symbol = format_ident!("{}", request.symbol);
  let address = format!(
    "battlement-reactant/generated/{}.png",
    self::hex(&request.identity())
  );
  let canvas_width = request.metadata.canvas.width as f32;
  let canvas_height = request.metadata.canvas.height as f32;
  let subject_x = request.metadata.subject.x as f32;
  let subject_y = request.metadata.subject.y as f32;
  let subject_width = request.metadata.subject.width as f32;
  let subject_height = request.metadata.subject.height as f32;
  let canvas = quote! {
    ::battlement_reactant::asset_generator::LogicalSize::new(#canvas_width, #canvas_height)
  };
  let subject = quote! {
    ::battlement_reactant::asset_generator::LogicalRect::new(
      #subject_x,
      #subject_y,
      #subject_width,
      #subject_height,
    )
  };
  let (handle, value, slices) = match request.kind {
    DeclarationKind::Background => (
      quote!(::battlement_reactant::asset_generator::BackgroundAsset),
      quote! {
        ::battlement_reactant::asset_generator::BackgroundAsset::__new(
          #address,
          #canvas,
          #subject,
        )
      },
      quote!(::core::option::Option::None),
    ),
    DeclarationKind::TextImage => (
      quote!(::battlement_reactant::asset_generator::TextImageAsset),
      quote! {
        ::battlement_reactant::asset_generator::TextImageAsset::__new(
          #address,
          #canvas,
          #subject,
        )
      },
      quote!(::core::option::Option::None),
    ),
    DeclarationKind::NineSlice => {
      let insets = request
        .metadata
        .slices
        .expect("validated nine-slice metadata");
      let top = insets.top as f32;
      let right = insets.right as f32;
      let bottom = insets.bottom as f32;
      let left = insets.left as f32;
      let raster_scale = request.metadata.raster_scale;
      let slices = quote! {
        ::battlement_reactant::asset_generator::LogicalInsets::new(
          #top,
          #right,
          #bottom,
          #left,
        )
      };
      (
        quote!(::battlement_reactant::asset_generator::NineSliceAsset),
        quote! {
          ::battlement_reactant::asset_generator::NineSliceAsset::__new(
            #address,
            #canvas,
            #subject,
            #slices,
            #raster_scale,
          )
        },
        quote!(::core::option::Option::Some(#slices)),
      )
    }
  };

  quote! {
    pub static #symbol: #handle = #value;

    ::battlement_reactant::__register_generated_asset!(
      ::battlement_reactant::asset_generator::AssetRegistration::__new(
        #address,
        #canvas,
        #subject,
        #slices,
        ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#symbol)),
      )
    );
  }
}

fn hex(bytes: &[u8]) -> String {
  const DIGITS: &[u8; 16] = b"0123456789abcdef";

  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
  }
  output
}
