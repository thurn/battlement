use std::{collections::BTreeMap, fmt::Write as _, fs, io::Write, path::Path, process::Command};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use battlement_reactant_asset_syntax::{ClipEdge, DeclarationKind, DependencyKind, Insets};

use crate::{AssetCatalog, CatalogAsset, Discovery, WorkReport, browser::BrowserRun};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const PREVIEW_PATH: &str = "Library/BattlementReactant/asset-preview/index.html";

pub(crate) fn open(
  project: &Path,
  discovery: &Discovery,
  catalog: &AssetCatalog,
  browser: &BrowserRun,
  report: &mut WorkReport,
) -> Result<()> {
  let assets = catalog
    .assets
    .iter()
    .map(|asset| (asset.request_identity, asset))
    .collect::<BTreeMap<_, _>>();
  let requests = browser
    .requests
    .iter()
    .map(|request| (request.address.as_str(), request))
    .collect::<BTreeMap<_, _>>();
  let mut cards = String::new();
  for (index, declaration) in discovery.assets.iter().enumerate() {
    let identity = declaration.request.identity();
    let asset = assets
      .get(&identity)
      .context("preview declaration was not present in the generated catalog")?;
    let request = requests
      .get(asset.address.as_str())
      .context("preview asset was not present in the browser result")?;
    let png = project
      .join(GENERATED_ROOT)
      .join("textures")
      .join(format!("{}.png", self::hex(&identity)));
    let bytes =
      fs::read(&png).with_context(|| format!("failed to read preview PNG {}", png.display()))?;
    report.files_opened += 1;
    report.generated_png_opens += 1;
    report.bytes_read += bytes.len() as u64;
    self::card(
      &mut cards,
      index,
      project,
      declaration,
      asset,
      request,
      &STANDARD.encode(bytes),
    )?;
  }
  let html = self::document(
    &cards,
    discovery.assets.len(),
    &format!(
      "{} {} · {} · renderer {}",
      browser.product, browser.version, browser.executable_sha256, browser.renderer_identity
    ),
  );
  self::write_and_open(project, &html, report)
}

pub(crate) fn open_empty(project: &Path, report: &mut WorkReport) -> Result<()> {
  self::write_and_open(
    project,
    &self::document(
      "<section class=\"empty\"><h2>No generated assets</h2><p>The selected rules package contains no asset declarations.</p></section>",
      0,
      "No renderer was started",
    ),
    report,
  )
}

fn card(
  html: &mut String,
  index: usize,
  project: &Path,
  declaration: &crate::DiscoveredAsset,
  asset: &CatalogAsset,
  request: &crate::browser::BrowserRequest,
  encoded_png: &str,
) -> Result<()> {
  let metadata = &declaration.request.metadata;
  let source = declaration
    .source_file
    .strip_prefix(project)
    .unwrap_or(&declaration.source_file)
    .to_string_lossy();
  let kind = self::kind_name(declaration.request.kind);
  let image = format!("data:image/png;base64,{encoded_png}");
  write!(
    html,
    "<article class=\"card\" data-kind=\"{}\"><header><p class=\"eyebrow\">{}</p><h2>{}</h2><p class=\"source\">{}:{}:{} · {}</p></header>",
    kind,
    kind,
    self::escape(&declaration.request.symbol),
    self::escape(&source),
    declaration.request.span.start_line,
    declaration.request.span.start_column,
    self::escape(&declaration.source_symbol),
  )?;
  write!(
    html,
    "<div class=\"visual\"><div class=\"checker\"><img src=\"{image}\" alt=\"Rendered {}\"></div>{}</div>",
    self::escape(&declaration.request.symbol),
    self::nine_slice(
      index,
      metadata.slices,
      metadata.raster_scale,
      &image,
      metadata.canvas.width,
      metadata.canvas.height
    ),
  )?;
  write!(
    html,
    "<dl><dt>Address</dt><dd><code>{}</code></dd><dt>Canonical hash</dt><dd><code>{}</code></dd><dt>Logical canvas</dt><dd>{} × {} px</dd><dt>Raster output</dt><dd>{} × {} px @ {}×</dd><dt>Subject bounds</dt><dd>x {} · y {} · {} × {} px</dd><dt>Alpha bounds</dt><dd>left {} · top {} · right {} · bottom {}</dd><dt>Allowed clipping</dt><dd>{}</dd><dt>Edge diagnostics</dt><dd>{}</dd></dl>",
    self::escape(&asset.address),
    self::hex(&asset.request_identity),
    metadata.canvas.width,
    metadata.canvas.height,
    request.width,
    request.height,
    metadata.raster_scale,
    metadata.subject.x,
    metadata.subject.y,
    metadata.subject.width,
    metadata.subject.height,
    request.alpha.left,
    request.alpha.top,
    request.alpha.right,
    request.alpha.bottom,
    self::clipping(&metadata.allowed_clipping),
    self::edges(request),
  )?;
  html.push_str("<section><h3>Authored properties</h3><ul class=\"properties\">");
  for paint in &declaration.request.paint {
    write!(
      html,
      "<li><code>{}: {}</code></li>",
      self::escape(&paint.property),
      self::escape(&paint.value)
    )?;
  }
  html.push_str("</ul></section><section><h3>Dependencies</h3><ul class=\"dependencies\">");
  if asset.dependencies.is_empty() {
    html.push_str("<li>None</li>");
  }
  for dependency in &asset.dependencies {
    write!(
      html,
      "<li><span>{}</span><code>{}</code><code>{}</code></li>",
      self::dependency_kind(dependency.kind),
      self::escape(&dependency.path),
      self::hex(&dependency.identity),
    )?;
  }
  html.push_str("</ul></section>");
  if !request.warnings.is_empty() {
    write!(
      html,
      "<p class=\"warnings\">Warnings: {}</p>",
      self::escape(&request.warnings.join(", "))
    )?;
  }
  html.push_str("</article>");
  Ok(())
}

fn nine_slice(
  index: usize,
  insets: Option<Insets>,
  scale: u8,
  image: &str,
  width: f64,
  height: f64,
) -> String {
  let Some(insets) = insets else {
    return String::new();
  };
  let minimum_width = insets.left + insets.right + 1.0;
  let minimum_height = insets.top + insets.bottom + 1.0;
  format!(
    "<div class=\"slice-tool\" data-slice><div class=\"slice-stage\" id=\"slice-{index}\" style=\"width:{width}px;height:{height}px;--image:url('{image}');--top:{}px;--right:{}px;--bottom:{}px;--left:{}px;--slice-top:{};--slice-right:{};--slice-bottom:{};--slice-left:{}\"><div class=\"sliced\"></div><i class=\"guide top\"></i><i class=\"guide right\"></i><i class=\"guide bottom\"></i><i class=\"guide left\"></i></div><label>Width <input data-width type=\"range\" min=\"{minimum_width}\" max=\"{}\" value=\"{width}\"></label><label>Height <input data-height type=\"range\" min=\"{minimum_height}\" max=\"{}\" value=\"{height}\"></label></div>",
    insets.top,
    insets.right,
    insets.bottom,
    insets.left,
    insets.top * f64::from(scale),
    insets.right * f64::from(scale),
    insets.bottom * f64::from(scale),
    insets.left * f64::from(scale),
    width * 3.0,
    height * 3.0,
  )
}

fn document(cards: &str, count: usize, browser: &str) -> String {
  format!(
    "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Reactant asset preview</title><style>{}</style></head><body><main><header class=\"hero\"><p class=\"eyebrow\">Battlement · Reactant</p><h1>Generated asset gallery</h1><p>{count} declarations · {}</p></header><div class=\"gallery\">{cards}</div></main><script>{}</script></body></html>\n",
    STYLE,
    self::escape(browser),
    SCRIPT,
  )
}

fn write_and_open(project: &Path, html: &str, report: &mut WorkReport) -> Result<()> {
  let preview = project.join(PREVIEW_PATH);
  let parent = preview.parent().expect("preview path has a parent");
  fs::create_dir_all(parent)
    .with_context(|| format!("failed to create preview directory {}", parent.display()))?;
  let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
  temporary.write_all(html.as_bytes())?;
  temporary.as_file_mut().sync_all()?;
  temporary
    .persist(&preview)
    .map_err(|error| error.error)
    .with_context(|| format!("failed to install asset preview {}", preview.display()))?;
  report.files_written += 1;
  self::open_file(&preview, report)
}

fn open_file(path: &Path, report: &mut WorkReport) -> Result<()> {
  #[cfg(target_os = "macos")]
  let status = Command::new("open").arg("-g").arg(path).status();
  #[cfg(target_os = "windows")]
  let status = Command::new("cmd")
    .args(["/C", "start", ""])
    .arg(path)
    .status();
  #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
  let status = Command::new("xdg-open").arg(path).status();
  report.subprocesses_started += 1;
  let status = status.context("failed to open the Reactant asset preview")?;
  if !status.success() {
    bail!("system URL opener exited with {status}");
  }
  Ok(())
}

fn edges(request: &crate::browser::BrowserRequest) -> String {
  let mut edges = Vec::new();
  if request.alpha.top == 0 {
    edges.push("top");
  }
  if request.alpha.right + 1 == request.width {
    edges.push("right");
  }
  if request.alpha.bottom + 1 == request.height {
    edges.push("bottom");
  }
  if request.alpha.left == 0 {
    edges.push("left");
  }
  if edges.is_empty() {
    "paint does not touch the canvas edge".to_owned()
  } else {
    format!("paint touches {}", edges.join(", "))
  }
}

fn clipping(edges: &[ClipEdge]) -> String {
  if edges.is_empty() {
    return "none".to_owned();
  }
  edges
    .iter()
    .map(|edge| match edge {
      ClipEdge::Top => "top",
      ClipEdge::Right => "right",
      ClipEdge::Bottom => "bottom",
      ClipEdge::Left => "left",
    })
    .collect::<Vec<_>>()
    .join(", ")
}

fn kind_name(kind: DeclarationKind) -> &'static str {
  match kind {
    DeclarationKind::Background => "background",
    DeclarationKind::NineSlice => "nine-slice",
    DeclarationKind::TextImage => "text-image",
  }
}

fn dependency_kind(kind: DependencyKind) -> &'static str {
  match kind {
    DependencyKind::Image => "image",
    DependencyKind::Font => "font",
  }
}

fn escape(value: &str) -> String {
  value
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#39;")
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

const STYLE: &str = r#"
:root{color-scheme:dark;font-family:Inter,ui-sans-serif,system-ui,sans-serif;background:#0b0d10;color:#f5f7fa}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 10% 0,#202a38 0,transparent 32rem),#0b0d10}main{width:min(1180px,calc(100% - 32px));margin:auto;padding:56px 0}.hero{margin-bottom:30px}.hero h1{font-size:clamp(2.4rem,7vw,5.4rem);letter-spacing:-.055em;line-height:.95;margin:.15em 0}.hero p:last-child{color:#9da7b4}.eyebrow{text-transform:uppercase;letter-spacing:.16em;color:#7dd3fc;font-size:.72rem;font-weight:750}.gallery{display:grid;gap:22px}.card{border:1px solid #303844;background:#15191fdd;border-radius:22px;padding:24px;box-shadow:0 24px 70px #0006}.card header{border-bottom:1px solid #2b323d;padding-bottom:16px;margin-bottom:20px}.card h2{font-size:1.65rem;margin:.25rem 0}.source{color:#98a2b0;font-size:.82rem;overflow-wrap:anywhere}.visual{display:grid;grid-template-columns:minmax(220px,1fr) minmax(300px,1.2fr);gap:22px}.checker,.slice-stage{background-color:#eef1f4;background-image:linear-gradient(45deg,#c7ccd2 25%,transparent 25%),linear-gradient(-45deg,#c7ccd2 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#c7ccd2 75%),linear-gradient(-45deg,transparent 75%,#c7ccd2 75%);background-size:20px 20px;background-position:0 0,0 10px,10px -10px,-10px 0}.checker{display:grid;place-items:center;min-height:220px;border-radius:14px;overflow:auto;padding:20px}.checker img{max-width:100%;height:auto;filter:drop-shadow(0 8px 18px #0005)}.slice-tool{display:grid;gap:12px;align-content:start}.slice-stage{position:relative;min-width:80px;min-height:80px;max-width:100%;border-radius:10px}.sliced{position:absolute;inset:0;border-style:solid;border-width:var(--top) var(--right) var(--bottom) var(--left);border-image-source:var(--image);border-image-slice:var(--slice-top) var(--slice-right) var(--slice-bottom) var(--slice-left) fill;border-image-repeat:stretch}.guide{position:absolute;background:#22d3ee;box-shadow:0 0 0 1px #083344}.guide.top{height:1px;left:0;right:0;top:var(--top)}.guide.right{width:1px;top:0;bottom:0;right:var(--right)}.guide.bottom{height:1px;left:0;right:0;bottom:var(--bottom)}.guide.left{width:1px;top:0;bottom:0;left:var(--left)}label{display:grid;grid-template-columns:54px 1fr;gap:10px;color:#aeb7c2;font-size:.8rem}input{width:100%}dl{display:grid;grid-template-columns:140px 1fr;gap:8px 18px;margin:24px 0}dt{color:#8792a1}dd{margin:0;min-width:0;overflow-wrap:anywhere}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;color:#c4f1ff;font-size:.78rem}.properties,.dependencies{padding:0;list-style:none;display:grid;gap:8px}.properties li,.dependencies li{background:#0d1117;border-radius:9px;padding:9px 11px}.dependencies li{display:grid;grid-template-columns:60px 1fr;gap:4px}.dependencies li code:last-child{grid-column:2;color:#7f8a98}.warnings{color:#fde68a}.empty{padding:40px;border:1px solid #303844;border-radius:20px;background:#15191f}@media(max-width:760px){.visual{grid-template-columns:1fr}dl{grid-template-columns:1fr}.dependencies li{grid-template-columns:1fr}.dependencies li code:last-child{grid-column:1}}
"#;

const SCRIPT: &str = r#"
document.querySelectorAll('[data-slice]').forEach(tool=>{const stage=tool.querySelector('.slice-stage');const width=tool.querySelector('[data-width]');const height=tool.querySelector('[data-height]');const update=()=>{stage.style.width=width.value+'px';stage.style.height=height.value+'px'};width.addEventListener('input',update);height.addEventListener('input',update);update()});
"#;
