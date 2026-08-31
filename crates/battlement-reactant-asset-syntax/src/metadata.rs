use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream};

use crate::{
  AssetRequest, ClipEdge, Compression, DEFAULT_RASTER_SCALE, DeclarationEnvelope, DeclarationKind,
  DependencyKind, Diagnostic, DiagnosticCategory, FilterMode, GeneratorMetadata, Insets,
  LocalDependency, LogicalRect, LogicalSize, PaintDeclaration, RawStatement, SourceSpan,
  StatementName, WrapMode,
  model::CanonicalPaintField,
  token::{Cursor, css_name},
  value::{self, ParsedRelation},
};

pub(crate) fn validate(envelope: DeclarationEnvelope) -> Result<AssetRequest, Diagnostic> {
  Builder::new(envelope).build()
}

struct Builder {
  envelope: DeclarationEnvelope,
  canvas: Option<LogicalSize>,
  subject: Option<LogicalRect>,
  slices: Option<Insets>,
  clipping: Vec<ClipEdge>,
  raster_scale: u8,
  filter_mode: FilterMode,
  wrap_mode: WrapMode,
  compression: Compression,
  font_file: Option<String>,
  paint: Vec<PaintDeclaration>,
  dependencies: Vec<LocalDependency>,
  background_layers: Option<usize>,
  blend_modes: Option<Vec<u8>>,
}

impl Builder {
  fn new(envelope: DeclarationEnvelope) -> Self {
    Self {
      envelope,
      canvas: None,
      subject: None,
      slices: None,
      clipping: Vec::new(),
      raster_scale: DEFAULT_RASTER_SCALE,
      filter_mode: FilterMode::Bilinear,
      wrap_mode: WrapMode::Clamp,
      compression: Compression::Lossless,
      font_file: None,
      paint: Vec::new(),
      dependencies: Vec::new(),
      background_layers: None,
      blend_modes: None,
    }
  }

  fn build(mut self) -> Result<AssetRequest, Diagnostic> {
    for statement in self.envelope.statements.clone() {
      match &statement.name {
        StatementName::Metadata(name) => self.metadata(name, &statement)?,
        StatementName::Property(name) => self.paint(name, &statement)?,
      }
    }
    self.validate_compositing()?;
    let canvas = self.canvas.ok_or_else(|| self.missing("@canvas"))?;
    self.validate_placement()?;
    self.validate_geometry(canvas)?;
    let subject = self.subject.unwrap_or(LogicalRect {
      x: 0.0,
      y: 0.0,
      width: canvas.width,
      height: canvas.height,
    });
    let default_subject = LogicalRect {
      x: 0.0,
      y: 0.0,
      width: canvas.width,
      height: canvas.height,
    };
    if self.subject.is_some() && subject == default_subject {
      return Err(self.at(
        DiagnosticCategory::RedundantDefault,
        "@subject",
        self.metadata_span("subject"),
      ));
    }
    self
      .paint
      .sort_by(|left, right| left.property.cmp(&right.property));
    self.dependencies.sort();
    self.dependencies.dedup();
    Ok(AssetRequest {
      symbol: self.envelope.symbol,
      kind: self.envelope.kind,
      metadata: GeneratorMetadata {
        canvas,
        subject,
        slices: self.slices,
        allowed_clipping: self.clipping,
        raster_scale: self.raster_scale,
        filter_mode: self.filter_mode,
        wrap_mode: self.wrap_mode,
        compression: self.compression,
        font_file: self.font_file,
      },
      paint: self.paint,
      dependencies: self.dependencies,
      span: self.envelope.span,
    })
  }

  fn metadata(&mut self, name: &str, statement: &RawStatement) -> Result<(), Diagnostic> {
    match name {
      "canvas" => {
        let values = self.pixels::<2>(statement, "@canvas")?;
        self.canvas = Some(LogicalSize {
          width: values[0],
          height: values[1],
        });
      }
      "subject" => {
        let values = self.pixels::<4>(statement, "@subject")?;
        self.subject = Some(LogicalRect {
          x: values[0],
          y: values[1],
          width: values[2],
          height: values[3],
        });
      }
      "slices" => {
        let values = self.pixels::<4>(statement, "@slices")?;
        self.slices = Some(Insets {
          top: values[0],
          right: values[1],
          bottom: values[2],
          left: values[3],
        });
      }
      "allow-clipping" => self.clipping = self.parse_clipping(statement)?,
      "raster-scale" => self.raster_scale = self.parse_scale(statement)?,
      "filter-mode" => {
        self.filter_mode = match self.keyword(statement, "@filter-mode")?.as_str() {
          "nearest" => FilterMode::Nearest,
          "bilinear" => return Err(self.redundant(statement, "@filter-mode")),
          _ => return Err(self.invalid(statement, "@filter-mode")),
        };
      }
      "wrap-mode" => {
        self.wrap_mode = match self.keyword(statement, "@wrap-mode")?.as_str() {
          "repeat" => WrapMode::Repeat,
          "clamp" => return Err(self.redundant(statement, "@wrap-mode")),
          _ => return Err(self.invalid(statement, "@wrap-mode")),
        };
      }
      "compression" => {
        self.compression = match self.keyword(statement, "@compression")?.as_str() {
          "lossy-low" => Compression::LossyLow,
          "lossy-normal" => Compression::LossyNormal,
          "lossy-high" => Compression::LossyHigh,
          "lossless" => return Err(self.redundant(statement, "@compression")),
          _ => return Err(self.invalid(statement, "@compression")),
        };
      }
      "font-file" => {
        let path = self.parse_font(statement)?;
        self.dependencies.push(LocalDependency {
          kind: DependencyKind::Font,
          path: path.clone(),
        });
        self.font_file = Some(path);
      }
      _ => {
        return Err(self.at(
          DiagnosticCategory::UnknownStatement,
          &format!("@{name}"),
          statement.span,
        ));
      }
    }
    Ok(())
  }

  fn paint(&mut self, name: &str, statement: &RawStatement) -> Result<(), Diagnostic> {
    if !property_allowed(self.envelope.kind, name) {
      return Err(self.at(DiagnosticCategory::UnknownStatement, name, statement.span));
    }
    let parsed = value::parse_property(name, &statement.value)
      .map_err(|error| self.at(error.category, name, statement.span))?;
    match &parsed.relation {
      Some(ParsedRelation::BackgroundLayers(count)) => {
        self.background_layers = Some(*count);
      }
      Some(ParsedRelation::BlendModes(modes)) => {
        self.blend_modes = Some(modes.clone());
      }
      None => {}
    }
    self
      .dependencies
      .extend(parsed.dependencies.into_iter().map(|path| LocalDependency {
        kind: DependencyKind::Image,
        path,
      }));
    for field in &parsed.fields {
      if self
        .paint
        .iter()
        .flat_map(|declaration| &declaration.canonical_fields)
        .any(|existing| existing.property == field.property)
      {
        return Err(self.at(DiagnosticCategory::DuplicateStatement, name, statement.span));
      }
    }
    let canonical_value = self::canonical_fields(&parsed.fields);
    self.paint.push(PaintDeclaration {
      property: name.to_owned(),
      value: statement.value.clone(),
      span: statement.span,
      canonical_value,
      canonical_fields: parsed
        .fields
        .into_iter()
        .map(|field| CanonicalPaintField {
          property: field.property,
          value: field.canonical,
        })
        .collect(),
    });
    Ok(())
  }

  fn validate_compositing(&mut self) -> Result<(), Diagnostic> {
    let Some(modes) = &self.blend_modes else {
      return Ok(());
    };
    let Some(layer_count) = self.background_layers else {
      let declaration = self
        .paint
        .iter()
        .find(|declaration| declaration.property == "background-blend-mode")
        .expect("parsed background blend declaration");
      return Err(self.at(
        DiagnosticCategory::InvalidValue,
        "background-blend-mode",
        declaration.span,
      ));
    };
    if modes.len() != 1 && modes.len() != layer_count {
      let declaration = self
        .paint
        .iter()
        .find(|declaration| declaration.property == "background-blend-mode")
        .expect("parsed background blend declaration");
      return Err(self.at(
        DiagnosticCategory::InvalidValue,
        "background-blend-mode",
        declaration.span,
      ));
    }
    let expanded = if modes.len() == 1 {
      vec![modes[0]; layer_count]
    } else {
      modes.clone()
    };
    let canonical = value::blend_canonical(&expanded);
    let declaration = self
      .paint
      .iter_mut()
      .find(|declaration| declaration.property == "background-blend-mode")
      .expect("parsed background blend declaration");
    declaration.canonical_value = canonical.clone();
    declaration
      .canonical_fields
      .first_mut()
      .expect("background blend canonical field")
      .value = canonical;
    Ok(())
  }

  fn validate_placement(&self) -> Result<(), Diagnostic> {
    match self.envelope.kind {
      DeclarationKind::Background => self.forbid(self.slices.is_some(), "@slices")?,
      DeclarationKind::NineSlice => {
        self.slices.ok_or_else(|| self.missing("@slices"))?;
      }
      DeclarationKind::TextImage => {
        self
          .font_file
          .as_ref()
          .ok_or_else(|| self.missing("@font-file"))?;
        self.require_paint("content")?;
        self.require_paint("font-size")?;
        self.forbid(self.slices.is_some(), "@slices")?;
      }
    }
    if self.envelope.kind != DeclarationKind::TextImage {
      self.forbid(self.font_file.is_some(), "@font-file")?;
    }
    Ok(())
  }

  fn validate_geometry(&self, canvas: LogicalSize) -> Result<(), Diagnostic> {
    if canvas.width <= 0.0
      || canvas.height <= 0.0
      || !scaled_integer(canvas.width, self.raster_scale)
      || !scaled_integer(canvas.height, self.raster_scale)
    {
      return Err(self.geometry("@canvas", self.metadata_span("canvas")));
    }
    if let Some(subject) = self.subject {
      let negative = [subject.x, subject.y, subject.width, subject.height]
        .into_iter()
        .any(|value| value < 0.0);
      if negative
        || subject.x + subject.width > canvas.width
        || subject.y + subject.height > canvas.height
      {
        return Err(self.geometry("@subject", self.metadata_span("subject")));
      }
    }
    if let Some(slices) = self.slices {
      let invalid_value = [slices.top, slices.right, slices.bottom, slices.left]
        .into_iter()
        .any(|value| value < 0.0 || !scaled_integer(value, self.raster_scale));
      if invalid_value
        || slices.top + slices.bottom >= canvas.height
        || slices.left + slices.right >= canvas.width
      {
        return Err(self.geometry("@slices", self.metadata_span("slices")));
      }
    }
    Ok(())
  }

  fn pixels<const N: usize>(
    &self,
    statement: &RawStatement,
    property: &str,
  ) -> Result<[f64; N], Diagnostic> {
    let mut cursor = value_cursor(statement);
    let mut values = [0.0; N];
    for value in &mut values {
      *value = cursor
        .pixels()
        .ok_or_else(|| self.invalid(statement, property))?;
    }
    if !cursor.is_empty() {
      return Err(self.invalid(statement, property));
    }
    Ok(values)
  }

  fn parse_scale(&self, statement: &RawStatement) -> Result<u8, Diagnostic> {
    let mut cursor = value_cursor(statement);
    let value = cursor
      .literal()
      .and_then(|value| value.parse::<u8>().ok())
      .filter(|value| (1..=8).contains(value));
    if !cursor.is_empty() || value.is_none() {
      return Err(self.invalid(statement, "@raster-scale"));
    }
    let value = value.expect("validated scale");
    if value == DEFAULT_RASTER_SCALE {
      return Err(self.redundant(statement, "@raster-scale"));
    }
    Ok(value)
  }

  fn parse_clipping(&self, statement: &RawStatement) -> Result<Vec<ClipEdge>, Diagnostic> {
    let mut cursor = value_cursor(statement);
    let mut edges = Vec::new();
    while !cursor.is_empty() {
      let edge = match css_name(&mut cursor)
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
      {
        Some("top") => ClipEdge::Top,
        Some("right") => ClipEdge::Right,
        Some("bottom") => ClipEdge::Bottom,
        Some("left") => ClipEdge::Left,
        _ => return Err(self.invalid(statement, "@allow-clipping")),
      };
      if edges
        .last()
        .is_some_and(|previous| edge_rank(*previous) >= edge_rank(edge))
      {
        return Err(self.at(
          DiagnosticCategory::InvalidClippingOrder,
          "@allow-clipping",
          statement.span,
        ));
      }
      edges.push(edge);
    }
    Ok(edges)
  }

  fn parse_font(&self, statement: &RawStatement) -> Result<String, Diagnostic> {
    let mut cursor = value_cursor(statement);
    let function = css_name(&mut cursor);
    let group = cursor.group(Delimiter::Parenthesis);
    if !cursor.is_empty() || !function.is_some_and(|name| name.eq_ignore_ascii_case("unity")) {
      return Err(self.invalid(statement, "@font-file"));
    }
    let mut arguments = Cursor::new(
      group
        .ok_or_else(|| self.invalid(statement, "@font-file"))?
        .0,
    );
    let path = arguments.string().filter(|path| !path.is_empty());
    if !arguments.is_empty() || path.is_none() {
      return Err(self.invalid(statement, "@font-file"));
    }
    crate::value::local_path(
      &path.expect("validated font path"),
      &["ttf", "otf", "woff2"],
    )
    .ok_or_else(|| self.invalid(statement, "@font-file"))
  }

  fn keyword(&self, statement: &RawStatement, property: &str) -> Result<String, Diagnostic> {
    let mut cursor = value_cursor(statement);
    let value = css_name(&mut cursor).map(|value| value.to_ascii_lowercase());
    if !cursor.is_empty() || value.is_none() {
      return Err(self.invalid(statement, property));
    }
    Ok(value.expect("validated keyword"))
  }

  fn require_paint(&self, property: &str) -> Result<(), Diagnostic> {
    if self.paint.iter().any(|paint| paint.property == property) {
      Ok(())
    } else {
      Err(self.missing(property))
    }
  }

  fn forbid(&self, present: bool, property: &str) -> Result<(), Diagnostic> {
    if present {
      Err(self.at(
        DiagnosticCategory::ForbiddenStatement,
        property,
        self.property_span(property),
      ))
    } else {
      Ok(())
    }
  }

  fn missing(&self, property: &str) -> Diagnostic {
    self.at(
      DiagnosticCategory::MissingStatement,
      property,
      self.envelope.span,
    )
  }

  fn invalid(&self, statement: &RawStatement, property: &str) -> Diagnostic {
    self.at(
      DiagnosticCategory::InvalidMetadata,
      property,
      statement.span,
    )
  }

  fn redundant(&self, statement: &RawStatement, property: &str) -> Diagnostic {
    self.at(
      DiagnosticCategory::RedundantDefault,
      property,
      statement.span,
    )
  }

  fn geometry(&self, property: &str, span: SourceSpan) -> Diagnostic {
    self.at(DiagnosticCategory::InvalidGeometry, property, span)
  }

  fn at(&self, category: DiagnosticCategory, property: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic {
      category,
      symbol: Some(self.envelope.symbol.clone()),
      property: Some(property.to_owned()),
      span,
    }
  }

  fn metadata_span(&self, name: &str) -> SourceSpan {
    self
      .envelope
      .statements
      .iter()
      .find_map(|statement| match &statement.name {
        StatementName::Metadata(value) if value == name => Some(statement.span),
        _ => None,
      })
      .unwrap_or(self.envelope.span)
  }

  fn property_span(&self, property: &str) -> SourceSpan {
    self.metadata_span(property.trim_start_matches('@'))
  }
}

fn value_cursor(statement: &RawStatement) -> Cursor {
  Cursor::new(TokenStream::from_str(&statement.value).expect("envelope value is a token stream"))
}

fn scaled_integer(value: f64, scale: u8) -> bool {
  value.is_finite() && (value * f64::from(scale)).fract() == 0.0
}

fn canonical_fields(fields: &[crate::value::ParsedField]) -> Vec<u8> {
  if let [field] = fields {
    return field.canonical.clone();
  }
  let mut bytes = Vec::new();
  bytes.extend(
    u32::try_from(fields.len())
      .expect("canonical paint field count overflow")
      .to_be_bytes(),
  );
  for field in fields {
    bytes.extend(
      u32::try_from(field.property.len())
        .expect("canonical paint property length overflow")
        .to_be_bytes(),
    );
    bytes.extend(field.property.as_bytes());
    bytes.extend(
      u32::try_from(field.canonical.len())
        .expect("canonical paint value length overflow")
        .to_be_bytes(),
    );
    bytes.extend(&field.canonical);
  }
  bytes
}

fn edge_rank(edge: ClipEdge) -> u8 {
  match edge {
    ClipEdge::Top => 0,
    ClipEdge::Right => 1,
    ClipEdge::Bottom => 2,
    ClipEdge::Left => 3,
  }
}

fn property_allowed(kind: DeclarationKind, property: &str) -> bool {
  const BOX: &[&str] = &[
    "background",
    "border",
    "border-width",
    "border-style",
    "border-color",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border-radius",
    "box-shadow",
    "clip-path",
    "mask",
    "opacity",
    "background-blend-mode",
    "isolation",
    "filter",
    "transform",
    "transform-origin",
  ];
  const TEXT: &[&str] = &[
    "content",
    "font-size",
    "font-style",
    "font-weight",
    "font-stretch",
    "line-height",
    "letter-spacing",
    "word-spacing",
    "text-align",
    "white-space",
    "color",
    "background",
    "background-clip",
    "-webkit-text-stroke",
    "text-shadow",
    "opacity",
    "filter",
    "transform",
    "transform-origin",
  ];
  match kind {
    DeclarationKind::Background | DeclarationKind::NineSlice => BOX.contains(&property),
    DeclarationKind::TextImage => TEXT.contains(&property),
  }
}
