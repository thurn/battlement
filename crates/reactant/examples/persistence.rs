//! Small real-storage consumer: persistence <path> [value | clear].

use reactant::{FilePersistenceBackend, PersistenceStore};
use std::{env, path::PathBuf, sync::Arc, time::Duration};

fn main() {
  let mut arguments = env::args().skip(1);
  let path = PathBuf::from(
    arguments
      .next()
      .expect("usage: persistence <path> [value | clear]"),
  );
  let store = PersistenceStore::<u32>::new(Some(path), Arc::new(FilePersistenceBackend));
  if let Some(value) = arguments.next() {
    if value == "clear" {
      store.clear();
    } else {
      store.update(value.parse().expect("value must be an unsigned integer"));
    }
  }
  assert!(
    store.wait_for_idle(Duration::from_secs(30)),
    "storage operation did not complete"
  );
  println!("{:?}", store.snapshot());
  if store.snapshot().error.is_some() {
    std::process::exit(1);
  }
}
