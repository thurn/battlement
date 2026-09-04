use std::{fs, path::Path};

use syn::{Expr, ImplItem, Item, Local, Pat, Stmt, visit::Visit};

#[test]
fn component_renders_use_only_hook_and_behavior_locals() {
  let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
  for entry in fs::read_dir(source).expect("chess-ui source directory") {
    let path = entry.expect("chess-ui source entry").path();
    if path.extension().and_then(|value| value.to_str()) != Some("rs") {
      continue;
    }
    let file = syn::parse_file(&fs::read_to_string(&path).expect("chess-ui source"))
      .expect("valid chess-ui Rust source");
    for item in file.items {
      let Item::Impl(implementation) = item else {
        continue;
      };
      if implementation
        .trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last())
        .is_none_or(|segment| segment.ident != "Component")
      {
        continue;
      }
      for item in implementation.items {
        let ImplItem::Fn(method) = item else {
          continue;
        };
        if method.sig.ident != "render" {
          continue;
        }
        assert!(
          !method
            .block
            .stmts
            .iter()
            .any(|statement| matches!(statement, Stmt::Expr(Expr::If(_), _))),
          "{} Component::render contains a top-level if",
          path.display()
        );
        for statement in &method.block.stmts {
          let Stmt::Local(local) = statement else {
            continue;
          };
          assert!(
            local
              .init
              .as_ref()
              .is_some_and(|initializer| is_hook_or_behavior(&initializer.expr)),
            "{} Component::render top-level local is not a hook or behavior binding",
            path.display()
          );
        }
        RenderLocals::new(&path).visit_block(&method.block);
      }
    }
  }
}

struct RenderLocals<'a> {
  path: &'a Path,
}

impl<'a> RenderLocals<'a> {
  fn new(path: &'a Path) -> Self {
    Self { path }
  }
}

impl<'ast> Visit<'ast> for RenderLocals<'_> {
  fn visit_local(&mut self, local: &'ast Local) {
    assert!(
      !contains_mutable_binding(&local.pat),
      "{} Component::render contains let mut",
      self.path.display()
    );
    syn::visit::visit_local(self, local);
  }
}

fn contains_mutable_binding(pattern: &Pat) -> bool {
  match pattern {
    Pat::Ident(pattern) => pattern.mutability.is_some(),
    Pat::Or(pattern) => pattern.cases.iter().any(contains_mutable_binding),
    Pat::Paren(pattern) => contains_mutable_binding(&pattern.pat),
    Pat::Reference(pattern) => contains_mutable_binding(&pattern.pat),
    Pat::Slice(pattern) => pattern.elems.iter().any(contains_mutable_binding),
    Pat::Struct(pattern) => pattern
      .fields
      .iter()
      .any(|field| contains_mutable_binding(&field.pat)),
    Pat::Tuple(pattern) => pattern.elems.iter().any(contains_mutable_binding),
    Pat::TupleStruct(pattern) => pattern.elems.iter().any(contains_mutable_binding),
    Pat::Type(pattern) => contains_mutable_binding(&pattern.pat),
    _ => false,
  }
}

fn is_hook_or_behavior(expression: &Expr) -> bool {
  match expression {
    Expr::Call(call) => match call.func.as_ref() {
      Expr::Path(path) => path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident.to_string().starts_with("use_")),
      _ => false,
    },
    Expr::MethodCall(call) => call.method == "bind",
    _ => false,
  }
}
