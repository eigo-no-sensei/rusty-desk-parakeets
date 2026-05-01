//! Tauri commands module

use std::path::PathBuf;
use std::time::Instant;

use parakeet_rs::{ExecutionConfig, ExecutionProvider, TimestampMode, Transcriber};
use tauri::State;
use tracing::{error, info, warn};

use crate::{AppState, TranscriptionResult};

/// Max chunk length fed to the encoder — 60 s at 16 kHz.
const CHUNK_SAMPLES: usize = 60 * 16_000;

/// Overlap between chunks to avoid cutting words — 1 s.
const OVERLAP_SAMPLES: usize = 1 * 16_000;

/// Initialize the model from the tdtv2.int8 folder
#[tauri::command]
pub async fn init_model(state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
    info!("Initializing Parakeet TDT model from: {:?}", state.model_dir);

    let load_start = Instant::now();

    if !state.model_dir.exists() {
        return Err(format!("Model directory not found: {:?}", state.model_dir));
    }

    let required_files = [
        "decoder_joint-model.int8.onnx",
        "encoder-model.int8.onnx",
        "tokenizer.json",
        "vocab.txt",
    ];

    for file in required_files {
        let file_path = state.model_dir.join(file);
        if !file_path.exists() {
            warn!("Model file not found: {:?}", file_path);
        }
    }

    let exec_config = ExecutionConfig::new()
    .with_execution_provider(ExecutionProvider::Cpu);

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

    let mut model_guard = state.model.lock().map_err(|e| e.to_string())?;
    *model_guard = Some(model);

    let load_duration = load_start.elapsed().as_secs_f64();
    info!("Model loaded in {:.2}s", load_duration);

    Ok(TranscriptionResult {
        text: format!("Model loaded in {:.2}s", load_duration),
       duration_secs: load_duration,
       success: true,
       error: None,
    })
}

/// Transcribe an audio file, chunking into 60 s segments.
#[tauri::command]
pub async fn transcribe_audio(
    audio_path: String,
    state: State<'_, AppState>,
) -> Result<TranscriptionResult, String> {
    info!("Transcribing: {}", audio_path);

    let path = PathBuf::from(&audio_path);
    if !path.exists() {
        return Err(format!("Audio file not found: {}", audio_path));
    }

    let mut model_guard = state.model.lock().map_err(|e| e.to_string())?;
    let model = model_guard
    .as_mut()
    .ok_or_else(|| "Model not initialized. Call init_model first.".to_string())?;

    // Decode at native rate and channel count.
    // NOTE: The new audio.rs implementation using ffmpeg-sidecar forces
    // output to 16 kHz mono, so sample_rate and channels will be fixed values here.
    let decode_start = Instant::now();

    let audio = crate::audio::decode_audio(&path)
    .map_err(|e| format!("Failed to decode audio: {}", e))?;

    let samples = audio.samples;
    let sample_rate = audio.sample_rate;
    let channels = audio.channels;

    let total_samples = samples.len();
    let decode_duration = decode_start.elapsed().as_secs_f64();

    info!(
        "Decoded: {} samples @ {} Hz × {} ch ({:.1}s) in {:.2}s",
          total_samples,
          sample_rate,
          channels,
          total_samples as f64 / (sample_rate as f64 * channels as f64),
          decode_duration,
    );

    // Chunk and transcribe
    //
    // Note on scaling: Since audio.rs decodes to 16000 Hz, rate_scale is 1.0.
    // We keep the calculation logic here for robustness in case the decoder changes.
    let chunk_samples = CHUNK_SAMPLES;
    let overlap_samples = OVERLAP_SAMPLES;
    let step = chunk_samples.saturating_sub(overlap_samples);
    // Adjust time calculations to use 16_000.0 and 1 channel directly

    let transcribe_start = Instant::now();
    let mut parts: Vec<String> = Vec::new();
    let step = chunk_samples.saturating_sub(overlap_samples);
    let mut offset = 0;

    while offset < total_samples {
        let end = (offset + chunk_samples).min(total_samples);
        let chunk = samples[offset..end].to_vec();

        let time_start = offset as f64 / (sample_rate as f64 * channels as f64);
        let time_end = end as f64 / (sample_rate as f64 * channels as f64);

        info!(
            "Chunk {:.1}s–{:.1}s ({} samples)",
              time_start,
              time_end,
              chunk.len(),
        );

        // Fix: Cast u32 to u16 to match parakeet-rs signature
        match model.transcribe_samples(
            chunk,
            sample_rate,
            channels as u16,
            Some(TimestampMode::Sentences)
        ) {
            Ok(r) => {
                let text = r.text.trim().to_string();
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            Err(e) => {
                error!("Chunk failed at offset {}: {}", offset, e);
                return Err(format!(
                    "Transcription failed at {:.1}s: {}",
                    time_start,
                    e,
                ));
            }
        }

        if end == total_samples {
            break;
        }
        offset += step;
    }

    let transcribe_duration = transcribe_start.elapsed().as_secs_f64();
    let total_duration = decode_duration + transcribe_duration;
    let full_text = parts.join(" ");

    info!(
        "Done: {} chars from {} chunks in {:.2}s",
        full_text.len(),
          parts.len(),
          transcribe_duration,
    );

    let mut transcript_guard = state.last_transcript.lock().map_err(|e| e.to_string())?;
    *transcript_guard = Some(full_text.clone());

    Ok(TranscriptionResult {
        text: full_text,
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

/// Get the yt-dlp executable path from bundled resources
fn get_ytdlp_path() -> Result<PathBuf, String> {
    // Try to get yt-dlp from the executable directory's resources folder
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or_else(|| "Failed to get executable directory".to_string())?;
    
    let ytdlp_path = exe_dir.join("yt-dlp");
    
    // Also check with .exe extension on Windows
    #[cfg(target_os = "windows")]
    let ytdlp_path = if ytdlp_path.exists() {
        ytdlp_path
    } else {
        exe_dir.join("yt-dlp.exe")
    };
    
    if !ytdlp_path.exists() {
        return Err("yt-dlp not found. Please place yt-dlp in the resources folder.".to_string());
    }
    
    Ok(ytdlp_path)
}

/// Validate if a string is a valid URL
fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Transcribe a YouTube (or any yt-dlp supported) URL
#[tauri::command]
pub async fn transcribe_youtube(
    url: String,
    keep_file: bool,
    download_video: bool,
    state: State<'_, AppState>,
) -> Result<TranscriptionResult, String> {
    // Validate URL
    if !is_valid_url(&url) {
        return Err(format!("Invalid URL: {}", url));
    }
    
    info!("Transcribing YouTube URL: {}", url);
    
    // Get yt-dlp path
    let ytdlp_path = get_ytdlp_path()?;
    info!("Using yt-dlp: {:?}", ytdlp_path);
    
    // Get downloads directory
    let downloads_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find downloads directory".to_string())?;
    
    // Create temporary directory for download
    let temp_dir = tempfile::Builder::new()
        .prefix("parakeet_ytdlp_")
        .tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    let temp_dir_path = temp_dir.path();
    let output_template = temp_dir_path.join("%(title)s.%(ext)s");
    
    // Build yt-dlp arguments
    let mut ytdlp_args = vec![
        "-f".to_string(),
    ];
    
    if download_video {
        // Download best video+audio, with fallback
        ytdlp_args.push("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string());
    } else {
        // Download best audio only (m4a or mp3)
        ytdlp_args.push("bestaudio[ext=m4a]/bestaudio".to_string());
    }
    
    // Output to temp directory as WAV (for audio) or MP4 (for video)
    if download_video {
        ytdlp_args.extend([
            "--output".to_string(),
            output_template.to_string_lossy().to_string(),
            "-x".to_string(),  // Extract audio from video
            "--audio-format".to_string(),
            "wav".to_string(),
        ]);
    } else {
        ytdlp_args.extend([
            "--output".to_string(),
            output_template.to_string_lossy().to_string(),
            "--audio-format".to_string(),
            "wav".to_string(),
        ]);
    }
    
    ytdlp_args.push(url.clone());
    
    info!("Running yt-dlp with args: {:?}", ytdlp_args);
    
    // Run yt-dlp
    let download_start = Instant::now();
    let output = std::process::Command::new(&ytdlp_path)
        .args(&ytdlp_args)
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("yt-dlp error: {}", stderr);
        return Err(format!("Failed to download: {}", stderr));
    }
    
    let download_duration = download_start.elapsed().as_secs_f64();
    info!("Download completed in {:.2}s", download_duration);
    
    // Find the downloaded file
    let mut downloaded_file: Option<PathBuf> = None;
    for entry in std::fs::read_dir(temp_dir_path)
        .map_err(|e| format!("Failed to read temp directory: {}", e))?
    {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "wav" || ext_str == "mp4" || ext_str == "m4a" || ext_str == "mp3" {
                    downloaded_file = Some(path);
                    break;
                }
            }
        }
    }
    
    let audio_path = downloaded_file
        .ok_or_else(|| "No file was downloaded".to_string())?;
    
    info!("Downloaded file: {:?}", audio_path);
    
    // If keep_file or download_video, move to downloads folder
    let final_path = if keep_file || download_video {
        let filename = audio_path.file_name()
            .ok_or_else(|| "Invalid filename".to_string())?;
        let dest = downloads_dir.join(filename);
        
        std::fs::rename(&audio_path, &dest)
            .or_else(|_| std::fs::copy(&audio_path, &dest).map(|_| ()))
            .map_err(|e| format!("Failed to save to downloads: {}", e))?;
        
        info!("Saved to: {:?}", dest);
        dest
    } else {
        audio_path
    };
    
    // Now transcribe the downloaded file
    let path = PathBuf::from(&final_path);
    let mut model_guard = state.model.lock().map_err(|e| e.to_string())?;
    let model = model_guard
        .as_mut()
        .ok_or_else(|| "Model not initialized. Call init_model first.".to_string())?;
    
    let decode_start = Instant::now();
    let audio = crate::audio::decode_audio(&path)
        .map_err(|e| format!("Failed to decode audio: {}", e))?;
    
    let samples = audio.samples;
    let sample_rate = audio.sample_rate;
    let channels = audio.channels;
    
    let total_samples = samples.len();
    let decode_duration = decode_start.elapsed().as_secs_f64();
    
    info!(
        "Decoded: {} samples @ {} Hz × {} ch ({:.1}s) in {:.2}s",
        total_samples, sample_rate, channels,
        total_samples as f64 / (sample_rate as f64 * channels as f64),
        decode_duration,
    );
    
    // Chunk and transcribe
    let transcribe_start = Instant::now();
    let mut parts: Vec<String> = Vec::new();
    let step = CHUNK_SAMPLES.saturating_sub(OVERLAP_SAMPLES);
    let mut offset = 0;
    
    while offset < total_samples {
        let end = (offset + CHUNK_SAMPLES).min(total_samples);
        let chunk = samples[offset..end].to_vec();
        
        let time_start = offset as f64 / (sample_rate as f64 * channels as f64);
        let time_end = end as f64 / (sample_rate as f64 * channels as f64);
        
        info!(
            "Chunk {:.1}s–{:.1}s ({} samples)",
            time_start, time_end, chunk.len(),
        );
        
        match model.transcribe_samples(
            chunk,
            sample_rate,
            channels as u16,
            Some(TimestampMode::Sentences)
        ) {
            Ok(r) => {
                let text = r.text.trim().to_string();
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            Err(e) => {
                error!("Chunk failed at offset {}: {}", offset, e);
                return Err(format!(
                    "Transcription failed at {:.1}s: {}",
                    time_start, e,
                ));
            }
        }
        
        if end == total_samples {
            break;
        }
        offset += step;
    }
    
    let transcribe_duration = transcribe_start.elapsed().as_secs_f64();
    let total_duration = download_duration + decode_duration + transcribe_duration;
    let full_text = parts.join(" ");
    
    info!(
        "Done: {} chars from {} chunks in {:.2}s",
        full_text.len(), parts.len(), transcribe_duration,
    );
    
    let mut transcript_guard = state.last_transcript.lock().map_err(|e| e.to_string())?;
    *transcript_guard = Some(full_text.clone());
    
    // Cleanup temp directory
    if !keep_file && !download_video {
        drop(temp_dir); // temp_dir goes out of scope and gets cleaned up
    }
    
    Ok(TranscriptionResult {
        text: full_text,
        duration_secs: total_duration,
        success: true,
        error: None,
    })
}
