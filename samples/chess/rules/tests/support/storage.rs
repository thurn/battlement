use chess_rules::persistence;
use reactant_testing::MemoryPersistence;
use std::path::PathBuf;
use std::sync::Arc;

pub fn with_save(bytes: &[u8]) -> Arc<MemoryPersistence> {
  MemoryPersistence::with_file(
    PathBuf::from("memory").join(persistence::SAVE_FILE_NAME),
    bytes,
  )
}
