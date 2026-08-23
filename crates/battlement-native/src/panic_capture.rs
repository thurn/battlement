use std::{any::Any, backtrace::Backtrace, cell::RefCell, panic, sync::Once};

static INSTALL: Once = Once::new();

thread_local! {
    static LAST_PANIC: RefCell<Option<CapturedPanic>> = const { RefCell::new(None) };
}

struct CapturedPanic {
    location: Option<String>,
    backtrace: String,
}

pub fn prepare() {
    INSTALL.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |information| {
            LAST_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(CapturedPanic {
                    location: information.location().map(|location| {
                        format!(
                            "{}:{}:{}",
                            location.file(),
                            location.line(),
                            location.column()
                        )
                    }),
                    backtrace: Backtrace::force_capture().to_string(),
                });
            });
            previous(information);
        }));
    });
    LAST_PANIC.with(|slot| slot.borrow_mut().take());
}

pub fn describe(payload: &(dyn Any + Send)) -> String {
    let message = payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_owned());
    let captured = LAST_PANIC.with(|slot| slot.borrow_mut().take());
    let Some(captured) = captured else {
        return message;
    };

    let location = captured
        .location
        .map(|location| format!("\nlocation: {location}"))
        .unwrap_or_default();
    format!("{message}{location}\nbacktrace:\n{}", captured.backtrace)
}
