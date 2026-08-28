use std::{
  any::Any,
  cell::RefCell,
  fmt::Write as _,
  io::{self, Write as _},
  panic,
  sync::Once,
};

use backtrace::Backtrace;
use color_backtrace::{
  BacktracePrinter, Frame, Verbosity,
  termcolor::{Ansi, Color, ColorSpec, WriteColor},
};

const MAXIMUM_VISIBLE_FRAMES: usize = 32;

static INSTALL: Once = Once::new();

thread_local! {
    static LAST_PANIC: RefCell<Option<CapturedPanic>> = const { RefCell::new(None) };
}

struct CapturedPanic {
  location: Option<String>,
  backtrace: Backtrace,
}

pub fn prepare() {
  INSTALL.call_once(|| {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |information| {
      let location = information.location().map(|location| {
        format!(
          "{}:{}:{}",
          location.file(),
          location.line(),
          location.column()
        )
      });
      let backtrace = Backtrace::new();
      LAST_PANIC.with(|slot| {
        *slot.borrow_mut() = Some(CapturedPanic {
          location,
          backtrace,
        });
      });
      previous(information);
    }));
  });
  LAST_PANIC.with(|slot| slot.borrow_mut().take());
}

pub fn describe(operation: &str, payload: &(dyn Any + Send)) -> String {
  let message = self::panic_message(payload);
  let captured = LAST_PANIC.with(|slot| slot.borrow_mut().take());
  let Some(captured) = captured else {
    return format!("Rust panic in {operation}: {message}");
  };

  self::format_panic(operation, &message, &captured).unwrap_or_else(|error| {
    let mut fallback = format!("Rust panic in {operation}: {message}");
    if let Some(location) = captured.location {
      let _ = write!(fallback, "\nlocation: {location}");
    }
    let _ = write!(fallback, "\nbacktrace formatting failed: {error}");
    fallback
  })
}

fn format_panic(operation: &str, message: &str, captured: &CapturedPanic) -> io::Result<String> {
  let mut output = Ansi::new(Vec::new());
  let mut header = ColorSpec::new();
  header.set_fg(Some(Color::Red)).set_bold(true);
  output.set_color(&header)?;
  writeln!(output, "Rust panic in {operation}")?;
  output.reset()?;

  let mut label = ColorSpec::new();
  label.set_fg(Some(Color::Cyan)).set_intense(true);
  output.set_color(&label)?;
  write!(output, "Message:  ")?;
  output.reset()?;
  writeln!(output, "{message}")?;

  output.set_color(&label)?;
  write!(output, "Location: ")?;
  output.reset()?;
  writeln!(
    output,
    "{}",
    captured.location.as_deref().unwrap_or("<unknown>")
  )?;

  BacktracePrinter::new()
    .lib_verbosity(Verbosity::Medium)
    .strip_function_hash(true)
    .add_frame_filter(Box::new(self::filter_frames))
    .print_trace(&captured.backtrace, &mut output)?;
  output.reset()?;
  Ok(String::from_utf8_lossy(&output.into_inner()).into_owned())
}

fn filter_frames(frames: &mut Vec<&Frame>) {
  frames.retain(|frame| !self::is_internal_frame(frame));
  frames.truncate(MAXIMUM_VISIBLE_FRAMES);
}

fn is_internal_frame(frame: &Frame) -> bool {
  const PREFIXES: &[&str] = &[
    "<alloc::",
    "<core::",
    "<std::",
    "__rust",
    "___rust",
    "alloc::",
    "backtrace::",
    "battlement_native::adapter::",
    "battlement_native::panic_capture::",
    "battlement_engine_",
    "color_backtrace::",
    "core::",
    "std::",
    "test::",
  ];

  let Some(name) = frame.name.as_deref() else {
    return false;
  };
  if PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
    return true;
  }

  name.contains("battlement_native::engine::EngineFactory")
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
  payload
    .downcast_ref::<&str>()
    .map(|message| (*message).to_owned())
    .or_else(|| payload.downcast_ref::<String>().cloned())
    .unwrap_or_else(|| "non-string panic payload".to_owned())
    .replace('\u{1b}', "\\x1b")
}
