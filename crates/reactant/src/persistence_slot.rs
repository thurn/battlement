use std::{
  collections::HashMap,
  path::{Component, Path, PathBuf},
  sync::{Arc, Mutex, OnceLock, Weak},
};

use uuid::Uuid;

use crate::persistence_operation::{
  PersistenceBackend, PersistenceCompletion, PersistenceId, PersistenceRequest,
};

static SLOTS: OnceLock<Mutex<HashMap<SlotKey, Weak<PersistenceSlot>>>> = OnceLock::new();

#[derive(Hash, PartialEq, Eq)]
enum Namespace {
  Shared(&'static str),
  Instance(usize),
}

#[derive(Hash, PartialEq, Eq)]
struct SlotKey(Namespace, PathBuf);

pub(crate) struct PersistenceSlot {
  backend: Arc<dyn PersistenceBackend>,
  path: PathBuf,
  state: Mutex<State>,
}

struct State {
  owner: Uuid,
  active: Option<PersistenceId>,
  pending: Option<Job>,
}

struct Job {
  request: PersistenceRequest,
  complete: PersistenceCompletion,
}

impl PersistenceSlot {
  pub(crate) fn claim(path: &Path, backend: Arc<dyn PersistenceBackend>, owner: Uuid) -> Arc<Self> {
    let path = self::normalize(path);
    let namespace = backend.namespace().map_or_else(
      || Namespace::Instance(Arc::as_ptr(&backend) as *const () as usize),
      Namespace::Shared,
    );
    let key = SlotKey(namespace, path.clone());
    let slot = {
      let mut slots = SLOTS.get_or_init(Mutex::default).lock().unwrap();
      slots.retain(|_, slot| slot.strong_count() != 0);
      if let Some(slot) = slots.get(&key).and_then(Weak::upgrade) {
        slot
      } else {
        let slot = Arc::new(Self {
          backend,
          path,
          state: Mutex::new(State {
            owner,
            active: None,
            pending: None,
          }),
        });
        slots.insert(key, Arc::downgrade(&slot));
        slot
      }
    };
    let obsolete = {
      let mut state = slot.state.lock().unwrap();
      state.owner = owner;
      state.pending.take()
    };
    if let Some(obsolete) = obsolete {
      obsolete.superseded();
    }
    slot
  }

  pub(crate) fn submit(
    self: &Arc<Self>,
    mut request: PersistenceRequest,
    complete: PersistenceCompletion,
  ) {
    request.path.clone_from(&self.path);
    let job = Job { request, complete };
    let (start, obsolete) = {
      let mut state = self.state.lock().unwrap();
      if state.owner != job.request.version.owner {
        (None, Some(job))
      } else if state.active.is_some() {
        (None, state.pending.replace(job))
      } else {
        state.active = Some(job.request.id);
        (Some(job), None)
      }
    };
    if let Some(obsolete) = obsolete {
      obsolete.superseded();
    }
    if let Some(job) = start {
      self.start(job);
    }
  }

  fn start(self: &Arc<Self>, job: Job) {
    let slot = self.clone();
    let id = job.request.id;
    self.backend.clone().start(
      job.request,
      Arc::new(move |completed, result| {
        if completed != id {
          return;
        }
        let next = {
          let mut state = slot.state.lock().unwrap();
          if state.active != Some(id) {
            return;
          }
          let next = state.pending.take();
          state.active = next.as_ref().map(|job| job.request.id);
          next
        };
        (job.complete)(id, result);
        if let Some(next) = next {
          slot.start(next);
        }
      }),
    );
  }
}

impl Job {
  fn superseded(self) {
    (self.complete)(
      self.request.id,
      Err("persistence slot owner was replaced".into()),
    );
  }
}

fn normalize(path: &Path) -> PathBuf {
  let mut result = PathBuf::new();
  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir if result.file_name().is_some_and(|name| name != "..") => {
        result.pop();
      }
      component => result.push(component.as_os_str()),
    }
  }
  result
}
