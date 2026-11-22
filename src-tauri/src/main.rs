// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `glaude --host`: the background process that owns agent processes.
    if std::env::args().any(|a| a == "--host") {
        glaude_lib::host::server::run();
    } else {
        glaude_lib::run()
    }
}
