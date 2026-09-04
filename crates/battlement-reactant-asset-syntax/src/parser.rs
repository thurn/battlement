use std::{collections::HashSet, error::Error, fmt, str::FromStr};

use proc_macro2::{Delimiter, TokenStream};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::token::{Cursor, SourceSpan, css_name};

/// Stable category for an authoring diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCategory {
  /// Input is not a valid Rust token tree.
  InvalidSyntax,
  /// The outer declaration or one statement is malformed.
  InvalidDeclaration,
  /// The generated static name is not a non-raw Rust identifier.
  InvalidIdentifier,
  /// One statement appears more than once.
  DuplicateStatement,
  /// A required statement is absent.
  MissingStatement,
  /// A recognized statement is invalid for this declaration kind.
  ForbiddenStatement,
  /// A statement name is outside the closed catalog.
  UnknownStatement,
  /// A generator metadata value has the wrong form or range.
  InvalidMetadata,
  /// Canvas, subject, or slice geometry is invalid.
  InvalidGeometry,
  /// Clipping edges are duplicated or out of order.
  InvalidClippingOrder,
  /// An explicitly authored value equals its defined default.
  RedundantDefault,
  /// A CSS value is outside the closed scalar grammar.
  InvalidValue,
  /// CSS calculation operands have incompatible dimensions or an invalid result.
  InvalidArithmetic,
  /// The complete request can be authored with native Reactant UI properties.
  NativeOnly,
}

impl DiagnosticCategory {
  /// Stable kebab-case identifier used by fixtures and command diagnostics.
  pub const fn code(self) -> &'static str {
    match self {
      Self::InvalidSyntax => "invalid-syntax",
      Self::InvalidDeclaration => "invalid-declaration",
      Self::InvalidIdentifier => "invalid-identifier",
      Self::DuplicateStatement => "duplicate-statement",
      Self::MissingStatement => "missing-statement",
      Self::ForbiddenStatement => "forbidden-statement",
      Self::UnknownStatement => "unknown-statement",
      Self::InvalidMetadata => "invalid-metadata",
      Self::InvalidGeometry => "invalid-geometry",
      Self::InvalidClippingOrder => "invalid-clipping-order",
      Self::RedundantDefault => "redundant-default",
      Self::InvalidValue => "invalid-value",
      Self::InvalidArithmetic => "invalid-arithmetic",
      Self::NativeOnly => "native-only",
    }
  }
}

/// Structured parser failure with stable context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
  /// Stable failure category.
  pub category: DiagnosticCategory,
  /// Generated static symbol, when it was known.
  pub symbol: Option<String>,
  /// Metadata or CSS property associated with the failure.
  pub property: Option<String>,
  /// Source span associated with the failure.
  pub span: SourceSpan,
  /// Native Reactant authoring replacement, when applicable.
  pub replacement: Option<String>,
}

impl fmt::Display for Diagnostic {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.category.code())?;
    if let Some(symbol) = &self.symbol {
      write!(formatter, " in {symbol}")?;
    }
    if let Some(property) = &self.property {
      write!(formatter, " at {property}")?;
    }
    if let Some(replacement) = &self.replacement {
      write!(formatter, "; use {replacement}")?;
    }
    write!(
      formatter,
      " ({}:{})",
      self.span.start_line, self.span.start_column
    )
  }
}

impl Error for Diagnostic {}

/// Generated handle family selected by the declaration at-rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
  /// Fixed-size background texture.
  Background,
  /// Resizable sliced background texture.
  NineSlice,
  /// Fixed-size advanced text texture.
  TextImage,
}

/// Normalized statement name from a declaration body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementName {
  /// Generator metadata at-rule without the leading `@`.
  Metadata(String),
  /// CSS property name.
  Property(String),
}

impl StatementName {
  fn key(&self) -> String {
    match self {
      Self::Metadata(name) => format!("@{name}"),
      Self::Property(name) => name.clone(),
    }
  }
}

/// One declaration-body statement with an unparsed value token tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawStatement {
  /// ASCII-lowercase normalized statement name.
  pub name: StatementName,
  /// Rust-token representation of the authored value.
  pub value: String,
  /// Statement-name source span.
  pub span: SourceSpan,
}

/// Parsed declaration boundary shared by macro expansion and host discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationEnvelope {
  /// Generated static name.
  pub symbol: String,
  /// Generated handle family.
  pub kind: DeclarationKind,
  /// Unique body statements in authored order.
  pub statements: Vec<RawStatement>,
  /// Complete declaration source span.
  pub span: SourceSpan,
}

pub(crate) fn parse(source: &str) -> Result<DeclarationEnvelope, Diagnostic> {
  let stream = TokenStream::from_str(source).map_err(|error| Diagnostic {
    category: DiagnosticCategory::InvalidSyntax,
    symbol: None,
    property: None,
    replacement: None,
    span: error.span().into(),
  })?;
  Parser::new(stream).envelope()
}

pub(crate) fn expand_family(source: &str) -> Result<Vec<String>, Diagnostic> {
  let stream = TokenStream::from_str(source).map_err(|error| Diagnostic {
    category: DiagnosticCategory::InvalidSyntax,
    symbol: None,
    property: None,
    replacement: None,
    span: error.span().into(),
  })?;
  FamilyParser::new(stream).sources()
}

struct FamilyParser {
  tokens: Cursor,
}

impl FamilyParser {
  fn new(stream: TokenStream) -> Self {
    Self {
      tokens: Cursor::new(stream),
    }
  }

  fn sources(mut self) -> Result<Vec<String>, Diagnostic> {
    let start = self.tokens.span();
    if !self.tokens.punct('@') {
      return Err(self.error(DiagnosticCategory::InvalidDeclaration, None, start));
    }
    let kind = css_name(&mut self.tokens)
      .ok_or_else(|| self.error(DiagnosticCategory::InvalidDeclaration, None, start))?;
    if !matches!(
      kind.to_ascii_lowercase().as_str(),
      "background" | "nine-slice" | "text-image"
    ) {
      return Err(self.error(DiagnosticCategory::InvalidDeclaration, None, start));
    }
    let (common, _) = self.tokens.group(Delimiter::Brace).ok_or_else(|| {
      self.error(
        DiagnosticCategory::InvalidDeclaration,
        None,
        self.tokens.span(),
      )
    })?;
    let common = Parser::new(TokenStream::new()).statements(common)?;
    let mut sources = Vec::new();
    while !self.tokens.is_empty() {
      let (symbol, symbol_span) = self.tokens.ident().ok_or_else(|| {
        self.error(
          DiagnosticCategory::InvalidIdentifier,
          None,
          self.tokens.span(),
        )
      })?;
      if !valid_rust_identifier(&symbol) {
        return Err(self.error(
          DiagnosticCategory::InvalidIdentifier,
          Some(&symbol),
          symbol_span,
        ));
      }
      let (member, _) = self.tokens.group(Delimiter::Brace).ok_or_else(|| {
        self.error(
          DiagnosticCategory::InvalidDeclaration,
          Some(&symbol),
          self.tokens.span(),
        )
      })?;
      let member = Parser::new(TokenStream::new()).statements(member)?;
      let mut statements = common.clone();
      for replacement in member {
        if let Some(existing) = statements
          .iter_mut()
          .find(|statement| statement.name.key() == replacement.name.key())
        {
          *existing = replacement;
        } else {
          statements.push(replacement);
        }
      }
      let body = statements
        .iter()
        .map(statement_source)
        .collect::<Vec<_>>()
        .join(" ");
      sources.push(format!("@{kind} {symbol} {{ {body} }}"));
    }
    if sources.is_empty() {
      return Err(self.error(DiagnosticCategory::InvalidDeclaration, None, start));
    }
    Ok(sources)
  }

  fn error(
    &self,
    category: DiagnosticCategory,
    symbol: Option<&str>,
    span: SourceSpan,
  ) -> Diagnostic {
    Diagnostic {
      category,
      symbol: symbol.map(str::to_owned),
      property: None,
      replacement: None,
      span,
    }
  }
}

fn statement_source(statement: &RawStatement) -> String {
  match &statement.name {
    StatementName::Metadata(name) => format!("@{name} {};", statement.value),
    StatementName::Property(name) => format!("{name}: {};", statement.value),
  }
}

struct Parser {
  tokens: Cursor,
  symbol: Option<String>,
}

impl Parser {
  fn new(stream: TokenStream) -> Self {
    Self {
      tokens: Cursor::new(stream),
      symbol: None,
    }
  }

  fn envelope(mut self) -> Result<DeclarationEnvelope, Diagnostic> {
    let start = self.tokens.span();
    let declaration = self.at_name(None)?;
    let kind = match declaration.to_ascii_lowercase().as_str() {
      "background" => DeclarationKind::Background,
      "nine-slice" => DeclarationKind::NineSlice,
      "text-image" => DeclarationKind::TextImage,
      _ => return Err(self.error(DiagnosticCategory::InvalidDeclaration, None, start)),
    };
    let (symbol, symbol_span) = self.tokens.ident().ok_or_else(|| {
      self.error(
        DiagnosticCategory::InvalidIdentifier,
        None,
        self.tokens.span(),
      )
    })?;
    if !valid_rust_identifier(&symbol) {
      return Err(self.error(DiagnosticCategory::InvalidIdentifier, None, symbol_span));
    }
    self.symbol = Some(symbol.clone());
    let (body, body_span) = self.tokens.group(Delimiter::Brace).ok_or_else(|| {
      self.error(
        DiagnosticCategory::InvalidDeclaration,
        None,
        self.tokens.span(),
      )
    })?;
    if !self.tokens.is_empty() {
      return Err(self.error(
        DiagnosticCategory::InvalidDeclaration,
        None,
        self.tokens.span(),
      ));
    }
    Ok(DeclarationEnvelope {
      symbol,
      kind,
      statements: self.statements(body)?,
      span: start.join(body_span),
    })
  }

  fn statements(&self, body: TokenStream) -> Result<Vec<RawStatement>, Diagnostic> {
    let mut body = Cursor::new(body);
    let mut statements = Vec::new();
    let mut seen = HashSet::new();
    while !body.is_empty() {
      let span = body.span();
      let name = if body.peek_punct('@') {
        body.punct('@');
        StatementName::Metadata(
          css_name(&mut body)
            .ok_or_else(|| self.error(DiagnosticCategory::InvalidDeclaration, None, span))?
            .to_ascii_lowercase(),
        )
      } else {
        let name = css_name(&mut body)
          .ok_or_else(|| self.error(DiagnosticCategory::InvalidDeclaration, None, span))?
          .to_ascii_lowercase();
        if !body.punct(':') {
          return Err(self.error(DiagnosticCategory::InvalidDeclaration, Some(&name), span));
        }
        StatementName::Property(name)
      };
      let property = name.key();
      if !seen.insert(property.clone()) {
        return Err(self.error(
          DiagnosticCategory::DuplicateStatement,
          Some(&property),
          span,
        ));
      }
      let mut value = TokenStream::new();
      while !body.is_empty() && !body.peek_punct(';') {
        value.extend([body.next().expect("nonempty cursor")]);
      }
      if value.is_empty() || !body.punct(';') {
        return Err(self.error(
          DiagnosticCategory::InvalidDeclaration,
          Some(&property),
          span,
        ));
      }
      statements.push(RawStatement {
        name,
        value: value.to_string(),
        span,
      });
    }
    Ok(statements)
  }

  fn at_name(&mut self, property: Option<&str>) -> Result<String, Diagnostic> {
    if !self.tokens.punct('@') {
      return Err(self.error(
        DiagnosticCategory::InvalidDeclaration,
        property,
        self.tokens.span(),
      ));
    }
    css_name(&mut self.tokens).ok_or_else(|| {
      self.error(
        DiagnosticCategory::InvalidDeclaration,
        property,
        self.tokens.span(),
      )
    })
  }

  fn error(
    &self,
    category: DiagnosticCategory,
    property: Option<&str>,
    span: SourceSpan,
  ) -> Diagnostic {
    Diagnostic {
      category,
      symbol: self.symbol.clone(),
      property: property.map(str::to_owned),
      replacement: None,
      span,
    }
  }
}

fn valid_rust_identifier(value: &str) -> bool {
  let mut characters = value.chars();
  let valid_start = characters
    .next()
    .is_some_and(|character| character == '_' || is_xid_start(character));
  valid_start
    && characters.all(is_xid_continue)
    && value != "_"
    && !RUST_KEYWORDS.contains(&value)
    && !value.starts_with("r#")
}

const RUST_KEYWORDS: &[&str] = &[
  "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
  "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
  "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
  "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
  "typeof", "unsized", "virtual", "yield", "try",
];
