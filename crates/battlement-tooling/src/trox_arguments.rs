use std::{collections::HashMap, mem};

use proc_macro2::{TokenStream, TokenTree};
use syn::{
  Block, Expr, ExprAssign, ExprClosure, FnArg, Ident, ItemFn, Local, Macro, Pat, Result, Token,
  parse::{Parse, ParseStream, Parser},
  punctuated::Punctuated,
  visit::{self, Visit},
};

/// An extraction/runtime mismatch at a Rust source location.
#[derive(Debug)]
pub struct ArgumentDiagnostic {
  pub line: usize,
  pub column: usize,
  pub placeholder: String,
}

struct ArgumentEntry {
  name: Ident,
  value: Option<Expr>,
}

#[derive(Default)]
struct Checker {
  scopes: Vec<HashMap<String, bool>>,
  diagnostics: Vec<ArgumentDiagnostic>,
}

impl Parse for ArgumentEntry {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    Ok(Self {
      name: input.parse()?,
      value: if input.peek(Token![=>]) {
        input.parse::<Token![=>]>()?;
        Some(input.parse()?)
      } else {
        None
      },
    })
  }
}

impl Checker {
  fn opaque(&self, expression: &Expr) -> bool {
    match expression {
      Expr::Call(call) => matches!(call.func.as_ref(), Expr::Path(path)
        if path.path.segments.last().is_some_and(|part| part.ident == "opaque")),
      Expr::Path(path) => path.path.get_ident().is_some_and(|name| self.binding(name)),
      Expr::Paren(value) => self.opaque(&value.expr),
      Expr::Group(value) => self.opaque(&value.expr),
      Expr::Reference(value) => self.opaque(&value.expr),
      Expr::MethodCall(call) => call.method == "clone" && self.opaque(&call.receiver),
      _ => false,
    }
  }

  fn binding(&self, name: &Ident) -> bool {
    self
      .scopes
      .iter()
      .rev()
      .find_map(|scope| scope.get(&name.to_string()))
      .copied()
      .unwrap_or(false)
  }

  fn bind(&mut self, pattern: &Pat, opaque: bool) {
    match pattern {
      Pat::Ident(pattern) => {
        self
          .scopes
          .last_mut()
          .unwrap()
          .insert(pattern.ident.to_string(), opaque);
      }
      Pat::Type(pattern) => self.bind(&pattern.pat, opaque),
      _ => {}
    }
  }

  fn arguments(&mut self, tokens: TokenStream) {
    let parser = Punctuated::<ArgumentEntry, Token![,]>::parse_terminated;
    // Internal @insert arms and macro definitions are not authoring call sites.
    let Ok(entries) = parser.parse2(tokens) else {
      return;
    };
    for entry in entries {
      let opaque = entry
        .value
        .as_ref()
        .map_or_else(|| self.binding(&entry.name), |value| self.opaque(value));
      if !opaque {
        continue;
      }
      // Trox's scanner recognizes only an unqualified inline marker.
      let inline = matches!(entry.value, Some(Expr::Call(ref call))
        if matches!(call.func.as_ref(), Expr::Path(path)
          if path.path.is_ident("opaque")));
      if !inline {
        let location = entry.name.span().start();
        self.diagnostics.push(ArgumentDiagnostic {
          line: location.line,
          column: location.column + 1,
          placeholder: entry.name.to_string(),
        });
      }
    }
  }

  fn nested_macros(&mut self, tokens: TokenStream) {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
      if let TokenTree::Group(group) = token {
        let argument_call = index.checked_sub(2).is_some_and(|start| {
          matches!(&tokens[start], TokenTree::Ident(name) if name == "tx_args")
            && matches!(&tokens[start + 1], TokenTree::Punct(mark) if mark.as_char() == '!')
        });
        if argument_call {
          self.arguments(group.stream());
        }
        self.nested_macros(group.stream());
      }
    }
  }
}

impl<'ast> Visit<'ast> for Checker {
  fn visit_item_fn(&mut self, function: &'ast ItemFn) {
    // Functions cannot capture enclosing locals; closures can.
    let enclosing = mem::take(&mut self.scopes);
    self.scopes.push(HashMap::new());
    for parameter in &function.sig.inputs {
      if let FnArg::Typed(parameter) = parameter {
        self.bind(&parameter.pat, false);
      }
    }
    visit::visit_item_fn(self, function);
    self.scopes = enclosing;
  }

  fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
    self.scopes.push(HashMap::new());
    for parameter in &closure.inputs {
      self.bind(parameter, false);
    }
    visit::visit_expr_closure(self, closure);
    self.scopes.pop();
  }

  fn visit_block(&mut self, block: &'ast Block) {
    self.scopes.push(HashMap::new());
    visit::visit_block(self, block);
    self.scopes.pop();
  }

  fn visit_local(&mut self, local: &'ast Local) {
    visit::visit_local(self, local);
    self.bind(
      &local.pat,
      local
        .init
        .as_ref()
        .is_some_and(|init| self.opaque(&init.expr)),
    );
  }

  fn visit_expr_assign(&mut self, value: &'ast ExprAssign) {
    visit::visit_expr_assign(self, value);
    if let Expr::Path(path) = value.left.as_ref()
      && let Some(name) = path.path.get_ident()
    {
      let opaque = self.opaque(&value.right);
      if let Some(scope) = self
        .scopes
        .iter_mut()
        .rev()
        .find(|scope| scope.contains_key(&name.to_string()))
      {
        scope.insert(name.to_string(), opaque);
      }
    }
  }

  fn visit_macro(&mut self, value: &'ast Macro) {
    if value
      .path
      .segments
      .last()
      .is_some_and(|part| part.ident == "tx_args")
    {
      self.arguments(value.tokens.clone());
    }
    self.nested_macros(value.tokens.clone());
  }
}

/// Checks local opaque arguments, including aliases, in ordinary Rust and UI macros.
/// This is a syntactic preflight; it does not infer return types across functions.
pub fn check_source(source: &str) -> Result<Vec<ArgumentDiagnostic>> {
  let file = syn::parse_file(source)?;
  let mut checker = Checker::default();
  checker.visit_file(&file);
  Ok(checker.diagnostics)
}
