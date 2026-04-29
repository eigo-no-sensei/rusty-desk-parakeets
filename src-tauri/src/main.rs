//! Parakeet TDT Transcription App - Main Entry Point

use parakeet_tdt_app_lib::{create_app_state, init_model, transcribe_audio, get_last_transcript};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("parakeet_tdt_app_lib=info".parse().unwrap()),
        )
        .init();

    info!("Starting Parakeet TDT Transcription App");

    // Get model directory - look for tdt.int8 in project root
    let exe_path = std::env::current_exe().unwrap_or_default();
    let app_dir = exe_path.parent().map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let model_dir = app_dir.join("tdt.int8");

    // If not found next to exe, try current directory
    let model_dir = if !model_dir.exists() {
        PathBuf::from("tdt.int8")
    } else {
        model_dir
    };

    info!("Model directory: {:?}", model_dir);

    // Create app state
    let app_state = create_app_state(model_dir);

    // Run Tauri app
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            init_model,
            transcribe_audio,
            get_last_transcript,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}