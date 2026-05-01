//! Parakeet TDT Transcription App - Main Entry Point

use parakeet_tdt_app_lib::create_app_state;
use tauri::Manager;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("parakeet_tdt_app_lib=info".parse().unwrap()),
        )
        .init();

    info!("Starting Parakeet TDT Transcription App");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let model_dir = app
                .path()
                .resource_dir()
                .expect("could not resolve resource dir")
                .join("tdtv2.int8");

            info!("Model directory: {:?}", model_dir);

            app.manage(create_app_state(model_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            parakeet_tdt_app_lib::commands::init_model,
            parakeet_tdt_app_lib::commands::transcribe_audio,
            parakeet_tdt_app_lib::commands::get_last_transcript,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
