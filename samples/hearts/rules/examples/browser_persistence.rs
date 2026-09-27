//! Browser contract entrypoint for the production typed store and IDBFS bridge.

#[cfg(target_os = "emscripten")]
#[path = "browser_persistence/fixture.rs"]
mod fixture;

fn main() {}
