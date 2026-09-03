use std::{cell::Cell, rc::Rc};

use battlement_builder::{
  builder,
  support::{self, Missing},
};

#[builder(support = support)]
struct Scale {
  #[builder(required)]
  width: f32,
  #[builder(required)]
  height: f32,
  #[builder(default = 1.0)]
  maximum: f32,
  roomy: Option<(f32, f32)>,
  caption: Option<String>,
}

#[builder(support = support)]
struct Frame<Child> {
  #[builder(required, into)]
  child: Rc<Child>,
  padding: f32,
}

#[builder(support = support)]
struct Borrowed<'a, T, const N: usize> {
  #[builder(required)]
  values: &'a [T; N],
  enabled: bool,
}

#[builder(support = support)]
struct GenericDefault<T> {
  values: Vec<T>,
}

#[builder(support = support)]
struct CallbackProps<'a> {
  #[builder(required)]
  on_change: Rc<dyn Fn(bool) -> bool + 'a>,
  on_focus: Option<Rc<dyn Fn() + 'a>>,
  #[allow(clippy::type_complexity)]
  formatter: Option<Rc<dyn for<'b> Fn(&'b str, usize) -> usize + 'a>>,
}

#[builder(support = support)]
struct RequiredOption {
  #[builder(required)]
  value: Option<String>,
}

#[builder(support = support)]
struct MarkerValue<T> {
  #[builder(required)]
  value: T,
}

#[test]
fn any_order_options_and_replacement_produce_the_completed_type() {
  let first: Scale = Scale::new()
    .roomy((800.0, 0.75))
    .height(600.0)
    .caption("hello")
    .width(800.0);
  let second: Scale = Scale::new()
    .width(800.0)
    .maximum(2.0)
    .height(600.0)
    .maximum(3.0)
    .caption(Some("world".to_owned()));
  assert_eq!(
    (first.width, first.height, first.maximum),
    (800.0, 600.0, 1.0)
  );
  assert_eq!(first.roomy, Some((800.0, 0.75)));
  assert_eq!(first.caption.as_deref(), Some("hello"));
  assert_eq!(second.maximum, 3.0);
  assert_eq!(second.caption.as_deref(), Some("world"));
  assert_eq!(second.roomy(None).roomy, None);
  let caption = "borrowed".to_owned();
  assert_eq!(first.caption(&caption).caption(None).caption, None);
  assert_eq!(RequiredOption::new().value(None).value, None);
}

#[test]
fn original_generic_arguments_are_inferred_without_extra_bounds() {
  struct NoDefault;
  let frame = Frame::new().padding(8.0).child(NoDefault);
  assert_eq!(frame.padding, 8.0);
  assert_eq!(Rc::strong_count(&frame.child), 1);
  let values = [1, 2, 3];
  let borrowed = Borrowed::new().enabled(true).values(&values);
  assert_eq!(borrowed.values, &values);
  assert!(borrowed.enabled);
  assert!(GenericDefault::<NoDefault>::new().values.is_empty());
  let value: MarkerValue<Missing<u8>> = MarkerValue::new().value(Missing::new());
  let _: Missing<u8> = value.value;
}

#[test]
fn callbacks_infer_parameters_capture_borrows_and_clear_without_invocation() {
  let calls = Cell::new(0);
  let props = CallbackProps::new()
    .on_focus(|| calls.set(calls.get() + 1))
    .formatter(|text, extra| text.len() + extra)
    .on_change(|value| !value);
  assert_eq!(calls.get(), 0);
  assert!((props.on_change)(false));
  assert_eq!((props.formatter.as_ref().unwrap())("hello", 2), 7);
  (props.on_focus.as_ref().unwrap())();
  assert_eq!(calls.get(), 1);
  let cleared = props.clear_on_focus().clear_formatter();
  assert!(cleared.on_focus.is_none());
  assert!(cleared.formatter.is_none());
}

#[test]
fn defaults_run_once_and_required_values_move_without_cloning() {
  thread_local! { static CALLS: Cell<u32> = const { Cell::new(0) }; }
  struct Owned(Rc<Cell<u32>>);
  impl Drop for Owned {
    fn drop(&mut self) {
      self.0.set(self.0.get() + 1);
    }
  }
  #[builder(support = support)]
  struct Props {
    #[builder(required)]
    owned: Owned,
    #[builder(default = CALLS.with(|calls| { calls.set(calls.get() + 1); calls.get() }))]
    serial: u32,
    #[builder(default = CALLS.with(|calls| { calls.set(calls.get() + 1); calls.get() }))]
    second: u32,
  }
  let drops = Rc::new(Cell::new(0));
  let props = Props::new().serial(99).owned(Owned(drops.clone()));
  CALLS.with(|calls| assert_eq!(calls.get(), 2));
  assert_eq!(props.serial, 99);
  assert_eq!(props.second, 2);
  assert_eq!(props.owned.0.get(), 0);
  drop(props);
  assert_eq!(drops.get(), 1);
}
