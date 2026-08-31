use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use syn::{Attribute, Item, ItemMacro, ItemMod, Macro, UseTree, visit::Visit};

use crate::{
  WorkReport,
  discovery::{DiscoveredAsset, Package},
  incremental::{FileFingerprint, fingerprint},
};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct SourceIndex {
  records: BTreeMap<String, SourceRecord>,
  #[serde(skip)]
  visited: BTreeSet<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SourceRecord {
  fingerprint: FileFingerprint,
  coordinate: String,
  crate_name: String,
  source_root: String,
  module: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  module_edges: Vec<ModuleEdge>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  declarations: Vec<CachedDeclaration>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ModuleEdge {
  source: PathBuf,
  module_dir: PathBuf,
  module: String,
  conditional: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CachedDeclaration {
  source_symbol: String,
  package: String,
  source_file: PathBuf,
  request_source: String,
}

pub(crate) fn scan_package(
  package: &Package,
  coordinate: &str,
  index: &mut SourceIndex,
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
    index,
    report,
    &mut seen,
    assets,
  )
}

impl SourceIndex {
  pub(crate) fn begin_run(&mut self) {
    self.visited.clear();
  }

  pub(crate) fn retain_visited(&mut self) {
    self.records.retain(|path, _| self.visited.contains(path));
  }

  fn reusable(
    &mut self,
    source: &Path,
    fingerprint: &FileFingerprint,
    module: &str,
    context: &ScanContext<'_>,
  ) -> Option<SourceRecord> {
    let key = self::normalized(source);
    self.visited.insert(key.clone());
    self
      .records
      .get(&key)
      .filter(|record| {
        record.fingerprint == *fingerprint
          && record.coordinate == context.coordinate
          && record.crate_name == context.crate_name
          && record.source_root == self::normalized(context.source_root)
          && record.module == module
      })
      .cloned()
  }

  fn insert(&mut self, source: &Path, record: SourceRecord) {
    let key = self::normalized(source);
    self.visited.insert(key.clone());
    self.records.insert(key, record);
  }
}

struct ScanContext<'a> {
  coordinate: &'a str,
  crate_name: &'a str,
  source_root: &'a Path,
}

struct FileCollector<'a> {
  source: &'a Path,
  context: &'a ScanContext<'a>,
  declarations: Vec<CachedDeclaration>,
  edges: Vec<ModuleEdge>,
}

#[allow(clippy::too_many_arguments)]
fn scan_file(
  source: &Path,
  module_dir: &Path,
  module: &str,
  conditional: bool,
  context: &ScanContext<'_>,
  index: &mut SourceIndex,
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
  let current = fingerprint(&source, report)
    .with_context(|| format!("failed to fingerprint Rust source {}", source.display()))?;
  let reused = index.reusable(&source, &current, module, context);
  let record = if let Some(record) = reused {
    record
  } else {
    let parsed = self::parse_file(
      &source,
      module_dir,
      module,
      conditional,
      context,
      current,
      report,
    )?;
    index.insert(&source, parsed.clone());
    parsed
  };
  self::append_declarations(&record.declarations, assets)?;
  for edge in &record.module_edges {
    self::scan_file(
      &edge.source,
      &edge.module_dir,
      &edge.module,
      edge.conditional,
      context,
      index,
      report,
      seen,
      assets,
    )?;
  }
  Ok(())
}

fn parse_file(
  source: &Path,
  module_dir: &Path,
  module: &str,
  conditional: bool,
  context: &ScanContext<'_>,
  fingerprint: FileFingerprint,
  report: &mut WorkReport,
) -> Result<SourceRecord> {
  let contents = fs::read_to_string(source)
    .with_context(|| format!("failed to read Rust source {}", source.display()))?;
  report.files_opened += 1;
  report.rust_source_opens += 1;
  report.bytes_read += contents.len() as u64;
  let file = syn::parse_file(&contents)
    .with_context(|| format!("failed to parse Rust source {}", source.display()))?;
  let mut collector = FileCollector {
    source,
    context,
    declarations: Vec::new(),
    edges: Vec::new(),
  };
  self::collect_items(
    &file.items,
    module_dir,
    module,
    conditional,
    report,
    &mut collector,
  )?;
  Ok(SourceRecord {
    fingerprint,
    coordinate: context.coordinate.to_owned(),
    crate_name: context.crate_name.to_owned(),
    source_root: self::normalized(context.source_root),
    module: module.to_owned(),
    module_edges: collector.edges,
    declarations: collector.declarations,
  })
}

fn collect_items(
  items: &[Item],
  module_dir: &Path,
  module: &str,
  conditional: bool,
  report: &mut WorkReport,
  collector: &mut FileCollector<'_>,
) -> Result<()> {
  for item in items {
    match item {
      Item::Macro(item_macro) => self::collect_macro(item_macro, module, conditional, collector)?,
      Item::Mod(item_module) => self::collect_module(
        item_module,
        module_dir,
        module,
        conditional,
        report,
        collector,
      )?,
      Item::Use(item_use) if self::use_mentions_generator(&item_use.tree) => bail!(
        "{} imports or reexports the asset generator; use the exact battlement_reactant::asset_generator::generate! path",
        collector.source.display()
      ),
      _ => {
        let mut visitor = NestedMacroVisitor::default();
        visitor.visit_item(item);
        if visitor.generator {
          bail!(
            "asset declaration in {} is nested; declarations must be top-level module items",
            collector.source.display()
          );
        }
      }
    }
  }
  Ok(())
}

fn collect_macro(
  item: &ItemMacro,
  module: &str,
  conditional: bool,
  collector: &mut FileCollector<'_>,
) -> Result<()> {
  if self::exact_generator(&item.mac) {
    if conditional || self::conditional(&item.attrs) {
      bail!(
        "asset declaration in {} is conditionally compiled",
        collector.source.display()
      );
    }
    let request_source = item.mac.tokens.to_string();
    let request = battlement_reactant_asset_syntax::parse(&request_source).with_context(|| {
      format!(
        "invalid asset declaration in {}",
        collector.source.display()
      )
    })?;
    let rust_symbol = if module.is_empty() {
      format!("{}::{}", collector.context.crate_name, request.symbol)
    } else {
      format!(
        "{}::{module}::{}",
        collector.context.crate_name, request.symbol
      )
    };
    collector.declarations.push(CachedDeclaration {
      source_symbol: format!("{}::{rust_symbol}", collector.context.coordinate),
      package: collector.context.coordinate.to_owned(),
      source_file: collector.source.to_owned(),
      request_source,
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
      collector.source.display()
    );
  }
  Ok(())
}

fn collect_module(
  item: &ItemMod,
  module_dir: &Path,
  module: &str,
  inherited_conditional: bool,
  report: &mut WorkReport,
  collector: &mut FileCollector<'_>,
) -> Result<()> {
  if item
    .attrs
    .iter()
    .any(|attribute| attribute.path().is_ident("path"))
  {
    bail!(
      "#[path] modules are unsupported during asset discovery in {}",
      collector.source.display()
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
    return self::collect_items(
      items,
      &module_dir.join(&name),
      &nested,
      conditional,
      report,
      collector,
    );
  }
  let flat = module_dir.join(format!("{name}.rs"));
  let nested_file = module_dir.join(&name).join("mod.rs");
  report.stat_calls += 2;
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
      collector.source.display()
    ),
  };
  collector.edges.push(ModuleEdge {
    source: next,
    module_dir: module_dir.join(name),
    module: nested,
    conditional,
  });
  Ok(())
}

fn append_declarations(
  declarations: &[CachedDeclaration],
  assets: &mut Vec<DiscoveredAsset>,
) -> Result<()> {
  for declaration in declarations {
    assets.push(DiscoveredAsset {
      source_symbol: declaration.source_symbol.clone(),
      package: declaration.package.clone(),
      source_file: declaration.source_file.clone(),
      request: battlement_reactant_asset_syntax::parse(&declaration.request_source).with_context(
        || {
          format!(
            "cached declaration {} is invalid",
            declaration.source_symbol
          )
        },
      )?,
    });
  }
  Ok(())
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

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
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
