use std::{cell::RefCell, ffi::CString, os::raw::c_char, sync::Arc};

use reactant::{FilePersistenceBackend, PersistenceStore};

thread_local! {
  static STORES: RefCell<Vec<PersistenceStore<u32>>> = const { RefCell::new(Vec::new()) };
  static SNAPSHOT: RefCell<CString> = RefCell::new(CString::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_reset() -> usize {
  STORES.with_borrow_mut(|stores| {
    let index = stores.len();
    stores.push(PersistenceStore::new(
      Some("/idbfs/match.json".into()),
      Arc::new(FilePersistenceBackend),
    ));
    index
  })
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_update(index: usize, value: u32) {
  STORES.with_borrow(|stores| stores[index].update(value));
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_clear(index: usize) {
  STORES.with_borrow(|stores| stores[index].clear());
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_retry(index: usize) {
  STORES.with_borrow(|stores| stores[index].retry());
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_snapshot(index: usize) -> *const c_char {
  let value = STORES.with_borrow(|stores| {
    let snapshot = stores[index].snapshot();
    serde_json::json!({
      "status": format!("{:?}", snapshot.status()),
      "desired": snapshot.desired,
      "durable": snapshot.durable,
      "owner": snapshot.version.owner,
      "revision": snapshot.version.revision,
      "committed": snapshot.committed.map(|version| version.revision),
      "error": snapshot.error,
    })
  });
  SNAPSHOT.with_borrow_mut(|snapshot| {
    *snapshot = CString::new(value.to_string()).unwrap();
    snapshot.as_ptr()
  })
}
