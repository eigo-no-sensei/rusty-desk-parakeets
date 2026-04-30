//! Parakeet TDT Transcription App - Rust Backend
//!
//! Handles audio decoding, model loading, and transcription via parakeet-rs.

mod audio;

use std::path::PathBuf;
use std::time::Instant;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use audio::AudioData;
use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode, Transcriber};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{error, info, warn};

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

/// Initialize the model from the tdt.int8 folder
#[tauri::command]
async fn init_model(state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
    info!("Initializing Parakeet TDT model from: {:?}", state.model_dir);

    let load_start = Instant::now();

    // Check if model directory exists
    if !state.model_dir.exists() {
        return Err(format!("Model directory not found: {:?}", state.model_dir));
    }

    // Check for required model files
    let required_files = [
        "decoder_joint-model.int8.onnx",
        "encoder-model.int8.onnx",
        "tokenizer.json",
        "vocab.txt",
    ];

    for file in required_files {
        let file_path = state.model_dir.join(file);
        if !file_path.exists() {
            warn!("Optional model file not found: {:?}", file_path);
        }
    }

    // Create execution config - use CPU by default
    // DirectML would be used on Windows if available
    let exec_config = ExecutionConfig::new()
        .with_execution_provider(ExecutionProvider::Cpu);

    // Load the model
    let model = match ParakeetTDT::from_pretrained(
        state.model_dir.to_string_lossy().as_ref(),
        Some(exec_config),
    ) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load model: {}", e);
            return Err(format!("Failed to load model: {}", e));
        }
    };

    // Store model in state
    let mut model_guard = state.model.lock().map_err(|e| e.to_string())?;
    *model_guard = Some(model);

    let load_duration = load_start.elapsed().as_secs_f64();
    info!("Model loaded successfully in {:.2}s", load_duration);

    Ok(TranscriptionResult {
        text: format!("Model loaded in {:.2}s", load_duration),
        duration_secs: load_duration,
        success: true,
        error: None,
    })
}

/// Transcribe an audio file
#[tauri::command]
async fn transcribe_audio(
    audio_path: String,
    state: State<'_, AppState>,
) -> Result<TranscriptionResult, String> {
    info!("Transcribing audio file: {}", audio_path);

    let path = PathBuf::from(&audio_path);

    // Check if file exists
    if !path.exists() {
        return Err(format!("Audio file not found: {}", audio_path));
    }

    // Get model from state
    let mut model_guard = state.model.lock().map_err(|e| e.to_string())?;
    let model = model_guard.as_mut().ok_or_else(|| {
        "Model not initialized. Call init_model first.".to_string()
    })?;

    // Decode audio
    let decode_start = Instant::now();
    let audio_data = match audio::decode_audio(&path) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to decode audio: {}", e);
            return Err(format!("Failed to decode audio: {}", e));
        }
    };

    let decode_duration = decode_start.elapsed().as_secs_f64();
    info!(
        "Decoded audio: {:.2}s @ {}Hz × {}ch",
        audio_data.duration_secs(),
        audio_data.sample_rate,
        audio_data.channels
    );

    // Transcribe
    let transcribe_start = Instant::now();

    // Use timestamp mode for word-level timestamps
    let timestamp_mode = Some(TimestampMode::Sentences);

    let result = match model.transcribe_samples(
        audio_data.samples,
        audio_data.sample_rate,
        audio_data.channels as u16,
        timestamp_mode,
    ) {
        Ok(r) => r,
        Err(e) => {
            error!("Transcription failed: {}", e);
            return Err(format!("Transcription failed: {}", e));
        }
    };

    let transcribe_duration = transcribe_start.elapsed().as_secs_f64();
    let total_duration = decode_duration + transcribe_duration;

    info!(
        "Transcription complete: {} chars in {:.2}s (decode: {:.2}s, transcribe: {:.2}s)",
        result.text.len(),
        total_duration,
        decode_duration,
        transcribe_duration
    );

    // Store result
    let mut transcript_guard = state.last_transcript.lock().map_err(|e| e.to_string())?;
    *transcript_guard = Some(result.text.clone());

    Ok(TranscriptionResult {
        text: result.text,
        duration_secs: total_duration,
        success: true,
        error: None,
    })
}

/// Get the last transcription result
#[tauri::command]
fn get_last_transcript(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.last_transcript.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

/// Create application state
pub fn create_app_state(model_dir: PathBuf) -> AppState {
    AppState {
        model: Mutex::new(None),
        model_dir,
        last_transcript: Mutex::new(None),
    }
}