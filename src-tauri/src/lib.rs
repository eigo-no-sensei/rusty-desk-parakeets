//! Parakeet TDT Transcription App - Rust Backend
//!
//! Handles audio decoding, model loading, and transcription via parakeet-rs.

mod audio;
mod commands;

use std::path::PathBuf;
use std::sync::Mutex;

use commands::{init_model, transcribe_audio, get_last_transcript};
use parakeet_rs::{ParakeetTDT, Transcriber};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{info};

/// Application state holding the model
pub struct AppState {
    /// The Parakeet TDT model
    model: Mutex<Option<ParakeetTDT>>,
    /// Model directory path
    model_dir: PathBuf,
    /// Last transcription result
    last_transcript: Mutex<Option<String>>,
}

/// Transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// The transcribed text
    pub text: String,
    /// Transcription time in seconds
    pub duration_secs: f64,
    /// Whether transcription was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Create application state
pub fn create_app_state(model_dir: PathBuf) -> AppState {
    AppState {
        model: Mutex::new(None),
        model_dir,
        last_transcript: Mutex::new(None),
    }
}