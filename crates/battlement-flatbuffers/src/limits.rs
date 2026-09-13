/// Largest accepted finished request or response.
pub const MAXIMUM_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum expanded traversal size accepted during verification.
pub const MAXIMUM_APPARENT_BYTES: usize = 64 * 1024 * 1024;
/// Maximum table nesting accepted during verification.
pub const MAXIMUM_TABLE_DEPTH: usize = 64;
/// Maximum number of table visits accepted during verification.
pub const MAXIMUM_TABLE_VISITS: usize = 1_000_000;
