use chess_rules::persistence;
use reactant_testing::MemoryPersistence;
use std::{path::PathBuf, rc::Rc};

pub fn with_save(bytes: &[u8]) -> Rc<MemoryPersistence> {
  MemoryPersistence::with_file(
    PathBuf::from("memory").join(persistence::SAVE_FILE_NAME),
    bytes,
  )
}
