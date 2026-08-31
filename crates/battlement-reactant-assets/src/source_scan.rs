use std::{
  collections::BTreeSet,
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use syn::{Attribute, Item, ItemMacro, ItemMod, Macro, UseTree, visit::Visit};

use crate::{
  WorkReport,
  discovery::{DiscoveredAsset, Package},
};

pub(crate) fn scan_package(
  package: &Package,
  coordinate: &str,
  report: &mut WorkReport,
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  let library = package
    .targets
    .iter()
    .find(|target| {
      target
        .kind
        .iter()
        .any(|kind| kind == "lib" || kind == "cdylib")
    })
    .with_context(|| format!("package {} does not define a library target", package.name))?;
  let mut seen = BTreeSet::new();
  let module_dir = library
    .src_path
    .parent()
    .context("library source has no parent directory")?;
  let source_root = package
    .manifest_path
    .parent()
    .context("package manifest has no parent directory")?
    .canonicalize()
    .with_context(|| format!("failed to resolve package root for {}", package.name))?;
  let crate_name = library.name.replace('-', "_");
  let context = ScanContext {
    coordinate,
    crate_name: &crate_name,
    source_root: &source_root,
  };
  self::scan_file(
    &library.src_path,
    module_dir,
    "",
    false,
    &context,
    report,
    &mut seen,
    assets,
  )
}

struct ScanContext<'a> {
  coordinate: &'a str,
  crate_name: &'a str,
  source_root: &'a Path,
}

#[allow(clippy::too_many_arguments)]
fn scan_file(
  source: &Path,
  module_dir: &Path,
  module: &str,
  conditional: bool,
  context: &ScanContext<'_>,
  report: &mut WorkReport,
  seen: &mut BTreeSet<PathBuf>,
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  let source = source
    .canonicalize()
    .with_context(|| format!("failed to open Rust source {}", source.display()))?;
  if !source.starts_with(context.source_root) {
    bail!(
      "Rust module {} escapes package root {}",
      source.display(),
      context.source_root.display()
    );
  }
  if !seen.insert(source.clone()) {
    return Ok(());
  }
  let contents = fs::read_to_string(&source)
    .with_context(|| format!("failed to read Rust source {}", source.display()))?;
  report.files_opened += 1;
  report.rust_source_opens += 1;
  report.bytes_read += contents.len() as u64;
  let file = syn::parse_file(&contents)
    .with_context(|| format!("failed to parse Rust source {}", source.display()))?;
  self::scan_items(
    &file.items,
    &source,
    module_dir,
    module,
    conditional,
    context,
    report,
    seen,
    assets,
  )
}

#[allow(clippy::too_many_arguments)]
fn scan_items(
  items: &[Item],
  source: &Path,
  module_dir: &Path,
  module: &str,
  conditional: bool,
  context: &ScanContext<'_>,
  report: &mut WorkReport,
  seen: &mut BTreeSet<PathBuf>,
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  for item in items {
    match item {
      Item::Macro(item_macro) => {
        self::scan_macro(item_macro, source, module, conditional, context, assets)?
      }
      Item::Mod(item_module) => self::scan_module(
        item_module,
        source,
        module_dir,
        module,
        conditional,
        context,
        report,
        seen,
        assets,
      )?,
      Item::Use(item_use) if self::use_mentions_generator(&item_use.tree) => bail!(
        "{} imports or reexports the asset generator; use the exact battlement_reactant::asset_generator::generate! path",
        source.display()
      ),
      _ => {
        let mut visitor = NestedMacroVisitor::default();
        visitor.visit_item(item);
        if visitor.generator {
          bail!(
            "asset declaration in {} is nested; declarations must be top-level module items",
            source.display()
          );
        }
      }
    }
  }
  Ok(())
}

fn scan_macro(
  item: &ItemMacro,
  source: &Path,
  module: &str,
  conditional: bool,
  context: &ScanContext<'_>,
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  if self::exact_generator(&item.mac) {
    if conditional || self::conditional(&item.attrs) {
      bail!(
        "asset declaration in {} is conditionally compiled",
        source.display()
      );
    }
    let request = battlement_reactant_asset_syntax::parse(&item.mac.tokens.to_string())
      .with_context(|| format!("invalid asset declaration in {}", source.display()))?;
    let rust_symbol = if module.is_empty() {
      format!("{}::{}", context.crate_name, request.symbol)
    } else {
      format!("{}::{module}::{}", context.crate_name, request.symbol)
    };
    assets.push(DiscoveredAsset {
      source_symbol: format!("{}::{rust_symbol}", context.coordinate),
      package: context.coordinate.to_owned(),
      source_file: source.to_owned(),
      request,
    });
    return Ok(());
  }
  let generator_like = item
    .mac
    .path
    .segments
    .last()
    .is_some_and(|segment| segment.ident == "generate")
    || item.mac.tokens.to_string().contains("asset_generator");
  if generator_like {
    let kind = if item.ident.is_some() {
      "macro wrapper"
    } else {
      "macro alias"
    };
    bail!(
      "unsupported asset-generator {kind} in {}; use the exact battlement_reactant::asset_generator::generate! path",
      source.display()
    );
  }
  Ok(())
}

#[allow(clippy::too_many_arguments)]
fn scan_module(
  item: &ItemMod,
  source: &Path,
  module_dir: &Path,
  module: &str,
  inherited_conditional: bool,
  context: &ScanContext<'_>,
  report: &mut WorkReport,
  seen: &mut BTreeSet<PathBuf>,
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  if item
    .attrs
    .iter()
    .any(|attribute| attribute.path().is_ident("path"))
  {
    bail!(
      "#[path] modules are unsupported during asset discovery in {}",
      source.display()
    );
  }
  let name = item.ident.to_string();
  let nested = if module.is_empty() {
    name.clone()
  } else {
    format!("{module}::{name}")
  };
  let conditional = inherited_conditional || self::conditional(&item.attrs);
  if let Some((_, items)) = &item.content {
    return self::scan_items(
      items,
      source,
      &module_dir.join(&name),
      &nested,
      conditional,
      context,
      report,
      seen,
      assets,
    );
  }
  let flat = module_dir.join(format!("{name}.rs"));
  let nested_file = module_dir.join(&name).join("mod.rs");
  let next = match (flat.is_file(), nested_file.is_file()) {
    (true, false) => flat,
    (false, true) => nested_file,
    (true, true) => bail!(
      "module {nested} has both {} and {}",
      flat.display(),
      nested_file.display()
    ),
    (false, false) => bail!(
      "module {nested} declared in {} has no source file",
      source.display()
    ),
  };
  self::scan_file(
    &next,
    &module_dir.join(name),
    &nested,
    conditional,
    context,
    report,
    seen,
    assets,
  )
}

fn exact_generator(value: &Macro) -> bool {
  value
    .path
    .segments
    .iter()
    .map(|segment| segment.ident.to_string())
    .eq(["battlement_reactant", "asset_generator", "generate"])
}

fn conditional(attributes: &[Attribute]) -> bool {
  attributes
    .iter()
    .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

fn use_mentions_generator(tree: &UseTree) -> bool {
  match tree {
    UseTree::Path(path) => {
      path.ident == "asset_generator"
        || path.ident == "generate"
        || self::use_mentions_generator(&path.tree)
    }
    UseTree::Name(name) => name.ident == "asset_generator" || name.ident == "generate",
    UseTree::Rename(rename) => {
      rename.ident == "asset_generator"
        || rename.ident == "generate"
        || rename.rename == "asset_generator"
        || rename.rename == "generate"
    }
    UseTree::Group(group) => group.items.iter().any(self::use_mentions_generator),
    UseTree::Glob(_) => false,
  }
}

#[derive(Default)]
struct NestedMacroVisitor {
  generator: bool,
}

impl<'ast> Visit<'ast> for NestedMacroVisitor {
  fn visit_macro(&mut self, value: &'ast Macro) {
    if self::exact_generator(value)
      || value
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "generate")
    {
      self.generator = true;
    }
    syn::visit::visit_macro(self, value);
  }
}
