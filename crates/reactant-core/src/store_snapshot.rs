use std::{any::Any, cell::RefCell};

use crate::{context, external_store::ExternalStore};

thread_local! {
  static CURRENT: RefCell<Option<Snapshots>> = const { RefCell::new(None) };
}

struct Snapshots {
  runtime: u64,
  entries: Vec<Box<dyn Any>>,
}

pub(crate) struct RenderSnapshots {
  owns: bool,
  previous: Option<Snapshots>,
}

pub(crate) fn enter(runtime: u64) -> RenderSnapshots {
  CURRENT.with(|current| {
    let mut current = current.borrow_mut();
    if current
      .as_ref()
      .is_some_and(|active| active.runtime == runtime)
    {
      RenderSnapshots {
        owns: false,
        previous: None,
      }
    } else {
      RenderSnapshots {
        owns: true,
        previous: current.replace(Snapshots {
          runtime,
          entries: Vec::new(),
        }),
      }
    }
  })
}

pub(crate) fn read<S: ExternalStore>(source: &S) -> S::Snapshot {
  let captured = CURRENT.with(|current| {
    current.borrow().as_ref().and_then(|entries| {
      entries.entries.iter().find_map(|entry| {
        let (candidate, snapshot) = entry.downcast_ref::<(S, S::Snapshot)>()?;
        (candidate == source).then(|| snapshot.clone())
      })
    })
  });
  if let Some(snapshot) = captured {
    return snapshot;
  }
  let snapshot = context::with_hooks_forbidden(|| source.snapshot());
  CURRENT.with(|current| {
    if let Some(entries) = current.borrow_mut().as_mut() {
      entries
        .entries
        .push(Box::new((source.clone(), snapshot.clone())));
    }
  });
  snapshot
}

impl Drop for RenderSnapshots {
  fn drop(&mut self) {
    if self.owns {
      CURRENT.with(|current| current.replace(self.previous.take()));
    }
  }
}
