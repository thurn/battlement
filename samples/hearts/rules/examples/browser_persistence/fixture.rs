use std::{cell::RefCell, ffi::CString, os::raw::c_char, sync::Arc};

use battlement_hearts_rules::{
  SavedMatch,
  domain::{HeartsState, IgnorePresentation, Intention, Phase, Seat, transition},
};
use reactant::{FilePersistenceBackend, GameVersion, PersistenceStore};

thread_local! {
  static STORES: RefCell<Vec<PersistenceStore<SavedMatch>>> = const { RefCell::new(Vec::new()) };
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
  STORES.with_borrow(|stores| stores[index].update(self::record(value)));
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
      "desired": snapshot.desired.as_ref().map(|value| value.revision),
      "durable": snapshot.durable.as_ref().map(|value| value.revision),
      "durable_record": snapshot.durable,
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

fn record(value: u32) -> SavedMatch {
  let mut state = HeartsState::new(u64::from(value));
  for _ in 0..value % 53 {
    let action = match state.phase() {
      Phase::Passing => {
        let seat = Seat::ALL
          .into_iter()
          .find(|seat| state.observe(*seat).pending_pass.is_none())
          .unwrap();
        Intention::SubmitPass {
          seat,
          cards: state.hand(seat)[..3].to_vec(),
        }
      }
      Phase::Playing { turn } => Intention::PlayCard {
        seat: turn,
        card: state.observe(turn).legal_plays[0],
      },
      _ => break,
    };
    transition::apply(&mut state, action, &mut IgnorePresentation).unwrap();
  }
  SavedMatch::capture(
    GameVersion {
      session: 1,
      revision: u64::from(value),
    },
    &state,
  )
}
