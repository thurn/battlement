use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine;
use battlement_reactant_asset_syntax::{DeclarationKind, DependencyKind};
use serde::Serialize;
use syn::LitStr;

use crate::{CatalogAsset, WorkReport, dependency::DependencyIndex};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenderDocument {
  pub(crate) key: String,
  pub(crate) width: f64,
  pub(crate) height: f64,
  pub(crate) viewport_width: u32,
  pub(crate) viewport_height: u32,
  pub(crate) scale: u8,
  declarations: Vec<Declaration>,
  dependencies: Vec<BlobDependency>,
  content: Option<String>,
  font_marker: Option<String>,
}

#[derive(Serialize)]
struct Declaration {
  property: String,
  value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BlobDependency {
  marker: String,
  mime_type: &'static str,
  base64: String,
}

pub(crate) fn build(
  asset: &CatalogAsset,
  key: String,
  project: &Path,
  dependencies: &mut DependencyIndex,
  report: &mut WorkReport,
) -> Result<RenderDocument> {
  let mut declarations = Vec::new();
  let mut content = None;
  for paint in &asset.request.paint {
    if paint.property == "content" {
      content = Some(
        syn::parse_str::<LitStr>(&paint.value)
          .context("validated text content is not a Rust string")?
          .value(),
      );
      continue;
    }
    let value =
      battlement_reactant_asset_syntax::serialize_property_value(&paint.property, &paint.value)
        .map_err(|error| anyhow::anyhow!("failed to serialize {}: {error}", paint.property))?;
    declarations.push(Declaration {
      property: paint.property.clone(),
      value,
    });
  }
  let mut blobs = Vec::new();
  let mut font_marker = None;
  for (index, dependency) in asset.dependencies.iter().enumerate() {
    let marker = format!("__battlement_reactant_dependency_{index}__");
    let bytes = dependencies.render_bytes(dependency, project, report)?;
    if dependency.kind == DependencyKind::Font {
      font_marker = Some(marker.clone());
    } else {
      let authored = format!("unity-url({:?})", dependency.path);
      let replacement = format!("url(\"{marker}\")");
      let mut replaced = false;
      for declaration in &mut declarations {
        if declaration.value.contains(&authored) {
          declaration.value = declaration.value.replace(&authored, &replacement);
          replaced = true;
        }
      }
      if !replaced {
        bail!(
          "image dependency {} has no rendered CSS reference",
          dependency.path
        );
      }
    }
    blobs.push(BlobDependency {
      marker,
      mime_type: self::mime_type(dependency.kind, &dependency.path)?,
      base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    });
  }
  if asset.request.kind == DeclarationKind::TextImage && font_marker.is_none() {
    bail!("text render {} has no local font dependency", asset.address);
  }
  let width = self::dimension(asset.request.metadata.canvas.width, "width")?;
  let height = self::dimension(asset.request.metadata.canvas.height, "height")?;
  Ok(RenderDocument {
    key,
    width,
    height,
    viewport_width: width.ceil() as u32,
    viewport_height: height.ceil() as u32,
    scale: asset.raster_scale,
    declarations,
    dependencies: blobs,
    content,
    font_marker,
  })
}

impl RenderDocument {
  pub(crate) fn data_url(&self) -> String {
    let html = format!(
      "<!doctype html><html lang=en><meta charset=utf-8><meta name=reactant-key content={:?}><style>html,body{{margin:0;width:100%;height:100%;overflow:hidden;background:transparent}}*{{box-sizing:border-box}}#asset{{position:absolute;box-sizing:border-box;margin:0;padding:0;border:0;text-decoration:none;background-color:transparent;direction:ltr;writing-mode:horizontal-tb}}</style><body><div id=asset></div></body></html>",
      self.key
    );
    format!(
      "data:text/html;base64,{}",
      base64::engine::general_purpose::STANDARD.encode(html)
    )
  }

  pub(crate) fn setup_expression(
    &self,
    subject: battlement_reactant_asset_syntax::LogicalRect,
  ) -> Result<String> {
    let input = serde_json::to_string(self)?;
    let subject = serde_json::json!({
      "x": subject.x,
      "y": subject.y,
      "width": subject.width,
      "height": subject.height,
    });
    Ok(format!(
      r#"(async()=>{{
const input={input};const subject={subject};const urls=[];const markers={{}};const imageLoads=[];
for(const dependency of input.dependencies){{const binary=atob(dependency.base64);const bytes=new Uint8Array(binary.length);for(let i=0;i<binary.length;i++)bytes[i]=binary.charCodeAt(i);const url=URL.createObjectURL(new Blob([bytes],{{type:dependency.mimeType}}));urls.push(url);markers[dependency.marker]=url;if(dependency.mimeType==='image/png'){{const image=new Image();image.src=url;imageLoads.push(image.decode());}}}}
window.__battlementReactantUrls=urls;
const asset=document.getElementById('asset');asset.style.left=subject.x+'px';asset.style.top=subject.y+'px';asset.style.width=subject.width+'px';asset.style.height=subject.height+'px';
if(input.fontMarker){{const face=new FontFace('BattlementReactantGenerated','url("'+markers[input.fontMarker]+'")');await face.load();document.fonts.add(face);asset.style.fontFamily='BattlementReactantGenerated';}}
for(const declaration of input.declarations){{let value=declaration.value;for(const marker in markers)value=value.split(marker).join(markers[marker]);asset.style.setProperty(declaration.property,value);if(!asset.style.getPropertyValue(declaration.property))throw new Error('browser rejected '+declaration.property+': '+value);if(declaration.property==='background-clip')asset.style.setProperty('-webkit-background-clip',value);}}
if(input.content!==null)asset.textContent=input.content;
await Promise.all(imageLoads);await document.fonts.ready;
if(input.fontMarker){{const size=asset.style.fontSize||'16px';if(!document.fonts.check(size+' BattlementReactantGenerated',input.content))throw new Error('local font face did not load');const raster=(text,font)=>{{const canvas=document.createElement('canvas');canvas.width=input.width*input.scale;canvas.height=input.height*input.scale;const context=canvas.getContext('2d');context.scale(input.scale,input.scale);context.font=font;context.fillText(text,0,subject.height*.8);return context.getImageData(0,0,canvas.width,canvas.height).data;}};const equal=(a,b)=>{{if(a.length!==b.length)return false;for(let i=0;i<a.length;i++)if(a[i]!==b[i])return false;return true;}};const font=getComputedStyle(asset).font;const actual=raster(input.content,font);if(equal(actual,raster(input.content,size+' __BattlementReactantUnavailable__')))throw new Error('text shaping fell back to an unavailable control family');const stripped=[...input.content].filter(character=>{{const code=character.codePointAt(0);return code!==0x200c&&code!==0x200d&&!(code>=0xfe00&&code<=0xfe0f)&&!(code>=0xe0100&&code<=0xe01ef);}}).join('');if(stripped!==input.content&&equal(actual,raster(stripped,font)))throw new Error('text shaping ignored a variation selector or joiner');const normalized=input.content.normalize('NFC');if(/\p{{Mark}}/u.test(input.content)&&normalized!==input.content&&!equal(actual,raster(normalized,font)))throw new Error('text shaping did not preserve a combining sequence');}}
const before=asset.getBoundingClientRect();await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));for(const animation of document.getAnimations()){{animation.pause();animation.currentTime=0;}}const after=asset.getBoundingClientRect();if(before.x!==after.x||before.y!==after.y||before.width!==after.width||before.height!==after.height)throw new Error('render layout did not stabilize');void asset.offsetWidth;return{{width:document.documentElement.clientWidth,height:document.documentElement.clientHeight}};
}})()"#,
      subject = subject
    ))
  }

  pub(crate) fn cleanup_expression() -> &'static str {
    "(()=>{for(const url of window.__battlementReactantUrls||[])URL.revokeObjectURL(url);window.__battlementReactantUrls=[];document.fonts.clear();})()"
  }
}

fn dimension(value: f64, name: &str) -> Result<f64> {
  if value <= 0.0 || value > f64::from(u32::MAX) || !value.is_finite() {
    bail!("logical canvas {name} {value} is outside browser viewport dimensions");
  }
  Ok(value)
}

fn mime_type(kind: DependencyKind, path: &str) -> Result<&'static str> {
  match kind {
    DependencyKind::Image => Ok("image/png"),
    DependencyKind::Font => match Path::new(path)
      .extension()
      .and_then(|extension| extension.to_str())
      .map(str::to_ascii_lowercase)
      .as_deref()
    {
      Some("ttf") => Ok("font/ttf"),
      Some("otf") => Ok("font/otf"),
      Some("woff2") => Ok("font/woff2"),
      _ => bail!("font dependency {path} has no supported MIME type"),
    },
  }
}
