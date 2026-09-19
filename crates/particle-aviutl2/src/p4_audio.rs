//! Seekable PCM16 WAV analysis. A band depends only on its requested time.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};

#[derive(Clone)]
struct AudioData {
    sample_rate: u32,
    samples: Vec<f32>,
}

struct Entry {
    length: u64,
    modified: Option<SystemTime>,
    audio: Arc<AudioData>,
}

static CACHE: OnceLock<Mutex<HashMap<PathBuf, Entry>>> = OnceLock::new();

pub(super) fn band_at(path: &Path, time: f64, band: usize) -> Option<f32> {
    if !time.is_finite() || time < 0.0 || band >= 10 {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 128 * 1024 * 1024 {
        return None;
    }
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let audio = {
        let mut guard = cache.lock().ok()?;
        let modified = meta.modified().ok();
        if let Some(hit) = guard.get(path) {
            if hit.length == meta.len() && hit.modified == modified {
                hit.audio.clone()
            } else {
                let data = Arc::new(parse_wav(&std::fs::read(path).ok()?)?);
                guard.insert(
                    path.to_path_buf(),
                    Entry {
                        length: meta.len(),
                        modified,
                        audio: data.clone(),
                    },
                );
                data
            }
        } else {
            let data = Arc::new(parse_wav(&std::fs::read(path).ok()?)?);
            if guard.len() >= 8 {
                guard.clear();
            }
            guard.insert(
                path.to_path_buf(),
                Entry {
                    length: meta.len(),
                    modified,
                    audio: data.clone(),
                },
            );
            data
        }
    };
    Some(analyze(&audio, time, band))
}

fn parse_wav(bytes: &[u8]) -> Option<AudioData> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut format = None;
    let mut pcm = None;
    let mut cursor = 12usize;
    while cursor.checked_add(8)? <= bytes.len() {
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().ok()?) as usize;
        let start = cursor + 8;
        let end = start.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        match &bytes[cursor..cursor + 4] {
            b"fmt " => {
                if size < 16 {
                    return None;
                }
                let chunk = &bytes[start..end];
                let encoding = u16::from_le_bytes(chunk[0..2].try_into().ok()?);
                let channels = u16::from_le_bytes(chunk[2..4].try_into().ok()?);
                let rate = u32::from_le_bytes(chunk[4..8].try_into().ok()?);
                let bits = u16::from_le_bytes(chunk[14..16].try_into().ok()?);
                if encoding != 1
                    || !(1..=2).contains(&channels)
                    || !(8000..=192000).contains(&rate)
                    || bits != 16
                {
                    return None;
                }
                format = Some((rate, channels as usize));
            }
            b"data" => pcm = Some(&bytes[start..end]),
            _ => {}
        }
        cursor = end.checked_add(size & 1)?;
    }
    let (sample_rate, channels) = format?;
    let raw = pcm?;
    let frame_size = channels * 2;
    if raw.len() % frame_size != 0 {
        return None;
    }
    let samples = raw
        .chunks_exact(frame_size)
        .map(|frame| {
            (0..channels)
                .map(|channel| {
                    i16::from_le_bytes([frame[channel * 2], frame[channel * 2 + 1]]) as f32
                        / 32768.0
                })
                .sum::<f32>()
                / channels as f32
        })
        .collect();
    Some(AudioData {
        sample_rate,
        samples,
    })
}

fn analyze(audio: &AudioData, time: f64, band: usize) -> f32 {
    const CENTERS: [f64; 10] = [
        40.0, 80.0, 160.0, 315.0, 630.0, 1250.0, 2500.0, 5000.0, 10000.0, 16000.0,
    ];
    let rate = audio.sample_rate as f64;
    let frequency = CENTERS[band].min(rate * 0.45);
    let center = (time * rate).floor() as usize;
    let n = 2048usize;
    let start = center.saturating_sub(n / 2);
    let omega = std::f64::consts::TAU * frequency / rate;
    let coeff = 2.0 * omega.cos();
    let mut previous = 0.0;
    let mut before = 0.0;
    for index in 0..n {
        let window = 0.5 - 0.5 * (std::f64::consts::TAU * index as f64 / (n - 1) as f64).cos();
        let sample = audio
            .samples
            .get(start.saturating_add(index))
            .copied()
            .unwrap_or(0.0) as f64
            * window;
        let current = sample + coeff * previous - before;
        before = previous;
        previous = current;
    }
    let power = previous * previous + before * before - coeff * previous * before;
    ((power.max(0.0).sqrt() * 4.0 / n as f64) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_wav_analysis_is_seek_stable() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + 8000 * 2).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(8000u32 * 2).to_le_bytes());
        for i in 0..8000 {
            let v = (12000.0 * (std::f64::consts::TAU * 80.0 * i as f64 / 8000.0).sin()) as i16;
            wav.extend_from_slice(&v.to_le_bytes());
        }
        let parsed = parse_wav(&wav).unwrap();
        let at_half = analyze(&parsed, 0.5, 1);
        assert!(at_half > 0.2);
        assert_eq!(at_half, analyze(&parsed, 0.5, 1));
        assert!(analyze(&parsed, 0.5, 1) > analyze(&parsed, 0.5, 9));
        assert!(parse_wav(b"invalid").is_none());
    }
}
