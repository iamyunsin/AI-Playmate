//! Tauri application entry point (shared between desktop and mobile builds).
//!
//! Platform-specific `main.rs` simply calls `run()`.

mod commands;
mod error;
mod state;

use tokio::sync::Mutex;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Bootstrap async state on the Tauri runtime
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                match AppState::init().await {
                    Ok(state) => {
                        handle.manage(Mutex::new(state));
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialise app state: {e}");
                        // Continue with a degraded state — commands will return errors
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::get_core_memory,
            commands::get_recent_messages,
            commands::search_memories,
            commands::update_user_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
