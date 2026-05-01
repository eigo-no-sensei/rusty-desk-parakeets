//! Audio decoding via ffmpeg-sidecar: any format FFmpeg supports → 16 kHz mono f32.

use anyhow::{anyhow, Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Decoded audio in its native format.
pub struct AudioData {
    /// Interleaved f32 samples, normalised to [-1, 1].
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
}

impl AudioData {
    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    /// Convert to 16 kHz mono — required by streaming models.
    pub fn to_16k_mono(&self) -> Vec<f32> {
        let mono = if self.channels == 1 {
            self.samples.clone()
        } else {
            self.samples
                .chunks(self.channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / self.channels as f32)
                .collect()
        };

        if self.sample_rate == 16_000 {
            mono
        } else {
            linear_resample(mono, self.sample_rate as usize, 16_000)
        }
    }
}

/// Decode any audio file FFmpeg can open.
///
/// Returns 16 kHz mono f32 samples.
pub fn decode_audio(path: &Path) -> Result<AudioData> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid UTF-8 in path"))?;

    let temp_path = make_temp_wav_path();
    let decode_result = decode_audio_inner(path_str, &temp_path);
    let cleanup_result = std::fs::remove_file(&temp_path);

    match (decode_result, cleanup_result) {
        (Ok(audio), _) => Ok(audio),
        (Err(err), Err(cleanup_err)) if cleanup_err.kind() != std::io::ErrorKind::NotFound => {
            Err(err).with_context(|| {
                format!(
                    "Also failed to remove temporary decoded file {}: {}",
                    temp_path.display(),
                    cleanup_err
                )
            })
        }
        (Err(err), _) => Err(err),
    }
}

fn decode_audio_inner(path_str: &str, temp_path: &Path) -> Result<AudioData> {
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid UTF-8 in temporary path"))?;

    let mut child = FfmpegCommand::new()
        .hide_banner()
        .overwrite()
        .no_video()
        .input(path_str)
        .args(["-ac", "1"])
        .args(["-ar", "16000"])
        .codec_audio("pcm_s16le")
        .output(temp_path_str)
        .spawn()
        .context("Failed to spawn FFmpeg")?;

    for event in child.iter()? {
        match event {
            FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, msg) => {
                tracing::error!("FFmpeg Error: {}", msg);
            }
            FfmpegEvent::Log(LogLevel::Warning, msg) => {
                tracing::warn!("FFmpeg Warning: {}", msg);
            }
            FfmpegEvent::Progress(p) => {
                tracing::debug!("FFmpeg Progress: {:?}", p);
            }
            _ => {}
        }
    }

    let wav_bytes = std::fs::read(temp_path).context("Failed to read temporary decoded WAV file")?;
    let pcm_bytes =
        wav_data_chunk(&wav_bytes).context("Failed to find PCM data chunk in FFmpeg WAV output")?;

    if pcm_bytes.is_empty() {
        return Err(anyhow!("No audio samples decoded"));
    }

    if pcm_bytes.len() % 2 != 0 {
        return Err(anyhow!(
            "Decoded PCM byte length is not divisible by 2: {} bytes",
            pcm_bytes.len()
        ));
    }

    let samples: Vec<f32> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .map(i16_to_f32)
        .collect();

    tracing::debug!(
        "Decoded {} samples @ 16000 Hz × 1 ch ({:.2}s) via FFmpeg",
        samples.len(),
        samples.len() as f64 / 16000.0,
    );

    Ok(AudioData {
        samples,
        sample_rate: 16_000,
        channels: 1,
    })
}

fn make_temp_wav_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!(
        "parakeet_decode_{}_{}.wav",
        std::process::id(),
        nanos
    ))
}

/// Return the bytes in a RIFF/WAV `data` chunk.
///
/// This is safer than skipping a fixed 44-byte header because FFmpeg may emit
/// optional chunks before `data`.
fn wav_data_chunk(wav: &[u8]) -> Result<&[u8]> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err(anyhow!("Not a RIFF/WAVE file"));
    }

    let mut pos = 12usize;
    while pos + 8 <= wav.len() {
        let chunk_id = &wav[pos..pos + 4];
        let chunk_len = u32::from_le_bytes([
            wav[pos + 4],
            wav[pos + 5],
            wav[pos + 6],
            wav[pos + 7],
        ]) as usize;
        pos += 8;

        if pos + chunk_len > wav.len() {
            return Err(anyhow!("Malformed WAV chunk exceeds file length"));
        }

        if chunk_id == b"data" {
            return Ok(&wav[pos..pos + chunk_len]);
        }

        // RIFF chunks are word-aligned; odd-sized chunks have one pad byte.
        pos += chunk_len + (chunk_len % 2);
    }

    Err(anyhow!("WAV data chunk not found"))
}

/// Convert signed 16-bit PCM to f32 in approximately [-1.0, 1.0].
///
/// Do not call `abs()` here: `i16::MIN.abs()` overflows because +32768 cannot
/// be represented as an i16.
#[inline]
fn i16_to_f32(s: i16) -> f32 {
    s as f32 / 32768.0
}

/// Linear interpolation resampler (mono).
pub fn linear_resample(input: Vec<f32>, from_hz: usize, to_hz: usize) -> Vec<f32> {
    if from_hz == to_hz {
        return input;
    }

    let ratio = from_hz as f64 / to_hz as f64;
    let out_len = ((input.len() as f64) / ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;
        let s0 = input.get(idx).copied().unwrap_or(0.0);
        let s1 = input.get(idx + 1).copied().unwrap_or(s0);
        out.push(s0 + (s1 - s0) * frac);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i16_min_does_not_panic() {
        assert_eq!(i16_to_f32(i16::MIN), -1.0);
        assert_eq!(i16_to_f32(0), 0.0);
        assert!(i16_to_f32(i16::MAX) > 0.999);
        assert!(i16_to_f32(i16::MAX) < 1.0);
    }

    #[test]
    fn finds_data_chunk_after_extra_chunk() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&0u32.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"JUNK");
        wav.extend_from_slice(&3u32.to_le_bytes());
        wav.extend_from_slice(b"abc");
        wav.push(0);
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(&[1, 2, 3, 4]);

        assert_eq!(wav_data_chunk(&wav).unwrap(), &[1, 2, 3, 4]);
    }
}
