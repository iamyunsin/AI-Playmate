// Tauri doesn't have a stable ABI on the frontend, so this re-export pattern
// lets the desktop binary and library share the same entry point.
// On mobile the `lib` crate-type is used; on desktop `main.rs` wraps it.

fn main() {
    ai_playmate_desktop_lib::run();
}
