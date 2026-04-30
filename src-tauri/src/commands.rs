//! Tauri commands module

use std::path::PathBuf;
use std::time::Instant;

use parakeet_rs::{ExecutionConfig, ExecutionProvider, TimestampMode, Transcriber};
use tauri::State;
use tracing::{error, info, warn};

use crate::{AppState, TranscriptionResult};

/// Initialize the model from the tdt.int8 folder
#[tauri::command]
pub async fn init_model(state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
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
    let exec_config = ExecutionConfig::new()
        .with_execution_provider(ExecutionProvider::Cpu);

    // Load the model
    let model = match parakeet_rs::ParakeetTDT::from_pretrained(
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
pub async fn transcribe_audio(
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
    let audio_data = match crate::audio::decode_audio(&path) {
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
pub fn get_last_transcript(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.last_transcript.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}