use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::encoder;

/// 判定麦克风「真的在采集」的能量阈值（RMS）。蓝牙耳机在 HFP 握手期间
/// CoreAudio 会照常回调，但送来的是全零静音，只看回调到达会误判成已就绪。
/// 真实麦克风即使在安静环境也有本底噪声，不会是精确的 0。
const READY_RMS_THRESHOLD: f32 = 0.00002;

fn rms(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let sum: f32 = data.iter().map(|s| s * s).sum();
    (sum / data.len() as f32).sqrt()
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
}

struct ActiveRecording {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

pub struct AudioRecorder {
    state: Mutex<RecordingState>,
    active: Mutex<Option<ActiveRecording>>,
    start_instant: Mutex<Option<Instant>>,
    /// 首个音频回调是否已到达。蓝牙耳机要先完成 HFP 握手才会送数据，
    /// 这段时间 CoreAudio 不回调，用这个标志告诉 UI 何时真正可以开口。
    audio_started: Arc<AtomicBool>,
}

// SAFETY: All fields are protected by Mutex. cpal::Stream is !Send/!Sync only
// due to platform marker types, but we never access the stream without holding
// the lock, so cross-thread usage is safe.
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RecordingState::Idle),
            active: Mutex::new(None),
            start_instant: Mutex::new(None),
            audio_started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.state.lock().unwrap() == RecordingState::Recording
    }

    pub fn elapsed_since_start(&self) -> Option<std::time::Duration> {
        self.start_instant.lock().unwrap().as_ref().map(|t| t.elapsed())
    }

    /// 本次录音是否已收到首个音频回调（即设备真正开始采集）。
    pub fn audio_started(&self) -> bool {
        self.audio_started.load(Ordering::SeqCst)
    }

    pub fn start(&self, device_name: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state == RecordingState::Recording {
            return Err("Already recording".to_string());
        }

        let device = crate::audio::find_input_device(device_name)
            .ok_or_else(|| "No input device available".to_string())?;

        let default_config = device.default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();
        let sample_format = default_config.sample_format();
        let config: cpal::StreamConfig = default_config.into();

        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        self.audio_started.store(false, Ordering::SeqCst);

        let stream = match sample_format {
            SampleFormat::F32 => {
                let sc = Arc::clone(&samples);
                let started = Arc::clone(&self.audio_started);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if !started.load(Ordering::Relaxed) && rms(data) > READY_RMS_THRESHOLD {
                            started.store(true, Ordering::SeqCst);
                        }
                        if let Ok(mut buf) = sc.try_lock() {
                            buf.extend_from_slice(data);
                        }
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None,
                )
            }
            SampleFormat::I16 => {
                let sc = Arc::clone(&samples);
                let started = Arc::clone(&self.audio_started);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if !started.load(Ordering::Relaxed) && !data.is_empty() {
                            let sum: f32 = data
                                .iter()
                                .map(|&s| {
                                    let f = s as f32 / 32768.0;
                                    f * f
                                })
                                .sum();
                            if (sum / data.len() as f32).sqrt() > READY_RMS_THRESHOLD {
                                started.store(true, Ordering::SeqCst);
                            }
                        }
                        if let Ok(mut buf) = sc.try_lock() {
                            buf.extend(data.iter().map(|&s| s as f32 / 32768.0));
                        }
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None,
                )
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        }.map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

        *state = RecordingState::Recording;
        *self.start_instant.lock().unwrap() = Some(Instant::now());
        let mut active = self.active.lock().unwrap();
        *active = Some(ActiveRecording { stream, samples, sample_rate, channels });

        Ok(())
    }

    pub fn stop(&self) -> Result<String, String> {
        let (samples_data, sample_rate, channels) = {
            let mut state = self.state.lock().unwrap();
            if *state != RecordingState::Recording {
                return Err("Not recording".to_string());
            }

            let mut active_guard = self.active.lock().unwrap();
            let recording = active_guard.take()
                .ok_or_else(|| "No active recording".to_string())?;

            // Explicitly pause before drop so CoreAudio calls AudioOutputUnitStop,
            // which releases the microphone and clears the macOS orange indicator.
            let _ = recording.stream.pause();
            drop(recording.stream);

            let samples = recording.samples.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *state = RecordingState::Idle;
            *self.start_instant.lock().unwrap() = None;
            (samples.clone(), recording.sample_rate, recording.channels)
        };

        if samples_data.is_empty() {
            return Err("No audio data captured".to_string());
        }

        // Mix to mono if multi-channel
        let mono = if channels > 1 {
            super::mix_to_mono(&samples_data, channels)
        } else {
            samples_data
        };

        // Resample to 16kHz
        let resampled = super::resample(&mono, sample_rate, 16_000);

        // Convert f32 [-1.0, 1.0] to i16
        let pcm: Vec<i16> = resampled.iter().map(|&s| {
            (s.clamp(-1.0, 1.0) * 32767.0) as i16
        }).collect();

        let flac_bytes = encoder::encode_flac(&pcm)?;
        Ok(encoder::audio_to_base64(&flac_bytes))
    }

    pub fn cancel(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state != RecordingState::Recording {
            return Err("Not recording".to_string());
        }

        let mut active_guard = self.active.lock().unwrap();
        if let Some(recording) = active_guard.take() {
            // Mirror stop()'s stream shutdown to release the mic and clear the
            // macOS orange indicator. Samples are dropped without encoding.
            let _ = recording.stream.pause();
            drop(recording.stream);
        }

        *state = RecordingState::Idle;
        *self.start_instant.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_recorder_is_idle() {
        let recorder = AudioRecorder::new();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_stop_when_not_recording_returns_error() {
        let recorder = AudioRecorder::new();
        assert!(recorder.stop().is_err());
    }

    #[test]
    fn test_mix_to_mono_stereo() {
        let stereo = vec![0.5, -0.5, 1.0, 0.0];
        let mono = crate::audio::mix_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < 1e-6);
        assert!((mono[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_resample_downsample() {
        let input = vec![0.0, 0.5, 1.0, 0.5];
        let output = crate::audio::resample(&input, 48_000, 16_000);
        assert!(!output.is_empty());
        assert!(output.len() < input.len());
    }

    #[test]
    fn test_resample_same_rate() {
        let input = vec![0.1, 0.2, 0.3];
        let output = crate::audio::resample(&input, 16_000, 16_000);
        assert_eq!(output, input);
    }

    #[test]
    fn test_elapsed_since_start_is_none_when_idle() {
        let recorder = AudioRecorder::new();
        assert!(recorder.elapsed_since_start().is_none());
    }

    #[test]
    fn test_cancel_when_not_recording_returns_error() {
        let recorder = AudioRecorder::new();
        assert!(recorder.cancel().is_err());
    }
}
