use std::{
  env, fs,
  io::Write,
  path::PathBuf,
  process::{Command, Stdio},
  time::SystemTime,
};

const IMPORTS: &str = "use battlement_builder::{builder, support};\n";
const ATTRIBUTE: &str = "#[builder(support = support)]";

#[test]
fn downstream_compiler_checks_required_props_and_invalid_declarations() {
  let compiler = Compiler::new();
  let required = format!("{ATTRIBUTE} struct Props {{ #[builder(required)] value: u32 }}");
  for (name, source, diagnostic) in [
    (
      "incomplete",
      format!("{required} fn accepts(_: Props) {{}} fn check() {{ accepts(Props::new()); }}"),
      "mismatched types",
    ),
    (
      "duplicate",
      format!("{required} fn check() {{ let _ = Props::new().value(1).value(2); }}"),
      "no method named `value`",
    ),
    (
      "missing_default",
      format!("struct Opaque; {ATTRIBUTE} struct Props {{ value: Opaque }}"),
      "Default",
    ),
    (
      "contradictory_default",
      format!("{ATTRIBUTE} struct Props {{ #[builder(required, default = 3)] value: u32 }}"),
      "required property cannot have a default",
    ),
    (
      "duplicate_attribute",
      format!(
        "{ATTRIBUTE} struct Props {{ #[builder(required)] #[builder(required)] value: u32 }}"
      ),
      "duplicate builder field option",
    ),
    (
      "unknown_attribute",
      format!("{ATTRIBUTE} struct Props {{ #[builder(whatever)] value: u32 }}"),
      "expected `required`",
    ),
    (
      "unknown_support",
      "#[builder(unknown = support)] struct Props;".to_owned(),
      "expected `support",
    ),
    (
      "tuple",
      format!("{ATTRIBUTE} struct Props(u32);"),
      "tuple structs",
    ),
    (
      "enum",
      format!("{ATTRIBUTE} enum Props {{ Value }}"),
      "requires a named-field or unit struct",
    ),
    (
      "field_cfg",
      format!("{ATTRIBUTE} struct Props {{ #[cfg(any())] value: u32 }}"),
      "conditionally compiled builder fields",
    ),
    (
      "self_bound",
      format!("trait Marker {{}} {ATTRIBUTE} struct Props where Self: Marker {{ value: u32 }}"),
      "Self-dependent",
    ),
    (
      "recursive",
      format!("{ATTRIBUTE} struct Props {{ #[builder(required)] next: Option<Box<Props>> }}"),
      "recursive builder declarations",
    ),
    (
      "new_collision",
      format!("{ATTRIBUTE} struct Props {{ new: u32 }}"),
      "conflicts with another generated builder method",
    ),
    (
      "clear_collision",
      format!(
        "{ATTRIBUTE} struct Props {{ clear_click: bool, click: Option<std::rc::Rc<dyn Fn()>> }}"
      ),
      "clearing method conflicts",
    ),
    (
      "clear_collision_reverse",
      format!(
        "{ATTRIBUTE} struct Props {{ click: Option<std::rc::Rc<dyn Fn()>>, clear_click: bool }}"
      ),
      "conflicts with another generated builder method",
    ),
    (
      "optional_forward_collision",
      format!(
        "{ATTRIBUTE} struct Props {{ click: Option<std::rc::Rc<dyn Fn()>>, click_optional: bool }}"
      ),
      "conflicts with another generated builder method",
    ),
    (
      "optional_forward_collision_reverse",
      format!(
        "{ATTRIBUTE} struct Props {{ click_optional: bool, click: Option<std::rc::Rc<dyn Fn()>> }}"
      ),
      "optional callback forwarding method conflicts",
    ),
    (
      "callback_into",
      format!(
        "{ATTRIBUTE} struct Props {{ #[builder(required, into)] click: std::rc::Rc<dyn Fn()> }}"
      ),
      "remove `into`",
    ),
    (
      "required_clear",
      format!(
        "{ATTRIBUTE} struct Props {{ #[builder(required)] click: Option<std::rc::Rc<dyn Fn()>> }} fn check() {{ let _ = Props::new().clear_click(); }}"
      ),
      "no method named `clear_click`",
    ),
    (
      "optional_callback_none",
      format!(
        "{ATTRIBUTE} struct Props {{ click: Option<std::rc::Rc<dyn Fn()>> }} fn check() {{ let _ = Props::new().click(None); }}"
      ),
      "Fn()",
    ),
    (
      "outer_drop",
      format!("{required} impl Drop for Props {{ fn drop(&mut self) {{}} }}"),
      "Drop",
    ),
  ] {
    compiler.fails(name, &source, diagnostic);
  }
  compiler.passes("generic_bounds", &format!(r#"
    trait Item {{ type Value; }}
    impl Item for u8 {{ type Value = u32; }}
    {ATTRIBUTE}
    struct Props<'a, T: Item, const N: usize> where T::Value: Copy {{
      #[builder(required)] value: &'a [T::Value; N],
      padding: f32,
    }}
    fn check() {{ let values = [3_u32, 4]; let _: Props<'_, u8, 2> = Props::new().padding(2.0).value(&values); }}
  "#));
  compiler.passes(
    "qualified_associated_bound",
    &format!(
      r#"
    trait Item {{ type Value; }}
    impl Item for u8 {{ type Value = u32; }}
    {ATTRIBUTE} struct Props<T: Item + Copy> {{
      #[builder(required)] value: <T as Item>::Value,
    }}
    fn check() {{ let _: Props<u8> = Props::new().value(3); }}
  "#
    ),
  );
  compiler.fails(
    "ambiguous_associated_bound",
    &format!(
      r#"
    trait Item {{ type Value; }}
    {ATTRIBUTE} struct Props<T: Item + Copy> {{
      #[builder(required)] value: T::Value,
    }}
  "#
    ),
    "spell this required associated type as",
  );
  compiler.passes("cfg_and_hygiene", &format!(r#"
    {ATTRIBUTE} #[cfg(any())] struct Absent {{ value: NotDeclared }}
    {ATTRIBUTE} #[cfg_attr(all(), cfg(any()), derive(Clone))] struct AlsoAbsent {{ value: NotDeclared }}
    {ATTRIBUTE} #[cfg_attr(all(), derive(Clone, Debug))] struct Present {{ value: u32 }}
    {ATTRIBUTE} struct Empty {{}}
    {ATTRIBUTE} struct Unit;
    {ATTRIBUTE} struct Props<__BuilderField0, __BuilderSignature0> {{
      #[builder(required)] r#type: __BuilderField0,
      #[builder(required)] value: __BuilderSignature0,
      __builder_value0: u8,
    }}
    fn check() {{
      let _ = Present::new().clone(); let _ = Empty::new(); let _ = Unit::new();
      let _ = Props::new().r#type(1_u8).value(2_u16).__builder_value0(3);
    }}
  "#));
  compiler.passes(
    "raw_internal_name",
    &format!(
      r#"
    {ATTRIBUTE} struct Props<r#__BuilderField0> {{
      #[builder(required)] value: r#__BuilderField0,
    }}
    fn check() {{ let _: Props<u32> = Props::new().value(3); }}
  "#
    ),
  );
  compiler.passes(
    "aliases_and_separate_options",
    &format!(
      r#"
    type Text = String;
    {ATTRIBUTE} struct Props {{
      #[builder(required)] #[builder(into)] text: Text,
      #[builder(default = String::from("default"), into)] other: String,
      optional: ::core::option::Option<::std::string::String>,
    }}
    fn check() {{ let _ = Props::new().optional("hello").text("world").other("new"); }}
  "#
    ),
  );
  compiler.passes(
    "mixed_raw_generic_spelling",
    &format!(
      r#"
    trait Item {{ type Value; }}
    impl Item for u8 {{ type Value = u32; }}
    {ATTRIBUTE} struct Props<r#T: Item> {{
      #[builder(required)] value: T::Value,
    }}
    fn check() {{ let _: Props<u8> = Props::new().value(3); }}
  "#
    ),
  );
  for count in [8, 16] {
    let fields = (0..count)
      .map(|index| format!("#[builder(required)] value{index}: u32,"))
      .collect::<String>();
    let setters = (0..count)
      .rev()
      .map(|index| format!(".value{index}({index}).optional({index})"))
      .collect::<String>();
    compiler.passes(&format!("required_{count}"), &format!("{ATTRIBUTE} struct Props {{ {fields} optional: u32 }} fn check() {{ let _: Props = Props::new(){setters}; }}"));
  }
}

struct Compiler {
  directory: PathBuf,
  deps: PathBuf,
  library: PathBuf,
}

impl Compiler {
  fn new() -> Self {
    let directory = env::temp_dir().join(format!(
      "battlement-builder-compiler-{}-{}",
      std::process::id(),
      SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let deps = env::current_exe().unwrap().parent().unwrap().to_owned();
    let library = fs::read_dir(&deps)
      .unwrap()
      .map(|entry| entry.unwrap().path())
      .filter(|path| {
        path
          .file_name()
          .unwrap()
          .to_string_lossy()
          .starts_with("libbattlement_builder-")
          && path
            .extension()
            .is_some_and(|extension| extension == "rlib")
      })
      .max_by_key(|path| path.metadata().unwrap().modified().unwrap())
      .expect("Cargo built the builder support library");
    Self {
      directory,
      deps,
      library,
    }
  }

  fn compile(&self, name: &str, source: &str) -> std::process::Output {
    let mut child = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
      .args([
        "--edition=2024",
        "--crate-type=lib",
        "--emit=metadata",
        "--cap-lints=allow",
        "--crate-name",
        name,
      ])
      .arg("--extern")
      .arg(format!("battlement_builder={}", self.library.display()))
      .arg("-L")
      .arg(format!("dependency={}", self.deps.display()))
      .arg("-o")
      .arg(self.directory.join(format!("{name}.rmeta")))
      .arg("-")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .unwrap();
    child
      .stdin
      .take()
      .unwrap()
      .write_all(format!("{IMPORTS}{source}").as_bytes())
      .unwrap();
    child.wait_with_output().unwrap()
  }

  fn passes(&self, name: &str, source: &str) {
    let output = self.compile(name, source);
    assert!(
      output.status.success(),
      "{name}: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }

  fn fails(&self, name: &str, source: &str, diagnostic: &str) {
    let output = self.compile(name, source);
    assert!(!output.status.success(), "{name} unexpectedly compiled");
    assert!(
      String::from_utf8_lossy(&output.stderr).contains(diagnostic),
      "{name}: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }
}

impl Drop for Compiler {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.directory);
  }
}
