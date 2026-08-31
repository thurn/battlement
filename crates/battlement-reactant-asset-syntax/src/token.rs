use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

/// One-based source coordinates for a token or declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
  /// First source line.
  pub start_line: usize,
  /// First source column.
  pub start_column: usize,
  /// Last source line.
  pub end_line: usize,
  /// Column immediately after the span.
  pub end_column: usize,
}

impl SourceSpan {
  pub(crate) fn join(self, other: Self) -> Self {
    Self {
      end_line: other.end_line,
      end_column: other.end_column,
      ..self
    }
  }
}

impl From<Span> for SourceSpan {
  fn from(span: Span) -> Self {
    let start = span.start();
    let end = span.end();
    Self {
      start_line: start.line,
      start_column: start.column + 1,
      end_line: end.line,
      end_column: end.column + 1,
    }
  }
}

pub(crate) struct Cursor {
  tokens: Vec<TokenTree>,
  index: usize,
}

impl Cursor {
  pub(crate) fn new(stream: TokenStream) -> Self {
    Self {
      tokens: stream.into_iter().collect(),
      index: 0,
    }
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.index == self.tokens.len()
  }

  pub(crate) fn next(&mut self) -> Option<TokenTree> {
    let token = self.tokens.get(self.index)?.clone();
    self.index += 1;
    Some(token)
  }

  pub(crate) fn span(&self) -> SourceSpan {
    self
      .tokens
      .get(self.index)
      .map(TokenTree::span)
      .unwrap_or(Span::call_site())
      .into()
  }

  pub(crate) fn peek_punct(&self, value: char) -> bool {
    matches!(self.tokens.get(self.index), Some(TokenTree::Punct(token)) if token.as_char() == value)
  }

  fn peek_name_hyphen(&self) -> bool {
    let Some(TokenTree::Punct(hyphen)) = self.tokens.get(self.index) else {
      return false;
    };
    let Some(TokenTree::Ident(identifier)) = self.tokens.get(self.index + 1) else {
      return false;
    };
    hyphen.as_char() == '-' && !identifier.to_string().is_empty()
  }

  pub(crate) fn punct(&mut self, value: char) -> bool {
    if self.peek_punct(value) {
      self.index += 1;
      true
    } else {
      false
    }
  }

  pub(crate) fn ident(&mut self) -> Option<(String, SourceSpan)> {
    match self.next()? {
      TokenTree::Ident(value) => Some((value.to_string(), value.span().into())),
      _ => {
        self.index -= 1;
        None
      }
    }
  }

  pub(crate) fn literal(&mut self) -> Option<String> {
    match self.next()? {
      TokenTree::Literal(value) => Some(value.to_string()),
      _ => {
        self.index -= 1;
        None
      }
    }
  }

  pub(crate) fn group(&mut self, delimiter: Delimiter) -> Option<(TokenStream, SourceSpan)> {
    match self.next()? {
      TokenTree::Group(value) if value.delimiter() == delimiter => {
        Some((value.stream(), value.span().into()))
      }
      _ => {
        self.index -= 1;
        None
      }
    }
  }

  pub(crate) fn pixels(&mut self) -> Option<f64> {
    let negative = self.punct('-');
    if !negative {
      self.punct('+');
    }
    let text = self.literal()?;
    let number = text.get(..text.len().checked_sub(2)?)?;
    if !text[text.len() - 2..].eq_ignore_ascii_case("px") || number.contains('_') {
      return None;
    }
    let value = number.parse::<f64>().ok()?;
    value
      .is_finite()
      .then_some(if negative { -value } else { value })
  }

  pub(crate) fn string(&mut self) -> Option<String> {
    let index = self.index;
    let text = self.literal()?;
    let Some(inner) = text
      .strip_prefix('"')
      .and_then(|text| text.strip_suffix('"'))
    else {
      self.index = index;
      return None;
    };
    let mut value = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
      let next = if character == '\\' {
        let Some(character) = characters.next() else {
          self.index = index;
          return None;
        };
        character
      } else {
        character
      };
      value.push(next);
    }
    Some(value)
  }
}

pub(crate) fn css_name(cursor: &mut Cursor) -> Option<String> {
  let mut name = String::new();
  if cursor.peek_name_hyphen() {
    cursor.punct('-');
    name.push('-');
  }
  name.push_str(&cursor.ident()?.0);
  while cursor.peek_name_hyphen() {
    cursor.punct('-');
    name.push('-');
    name.push_str(&cursor.ident()?.0);
  }
  Some(name)
}
