use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::{Arc, Mutex};

use super::encoder;

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
        }
    }

    pub fn is_recording(&self) -> bool {
        *self.state.lock().unwrap() == RecordingState::Recording
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

        let stream = match sample_format {
            SampleFormat::F32 => {
                let sc = Arc::clone(&samples);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
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
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
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
            (samples.clone(), recording.sample_rate, recording.channels)
        };

        if samples_data.is_empty() {
            return Err("No audio data captured".to_string());
        }

        // Mix to mono if multi-channel
        let mono = if channels > 1 {
            mix_to_mono(&samples_data, channels)
        } else {
            samples_data
        };

        // Resample to 16kHz
        let resampled = resample(&mono, sample_rate, 16_000);

        // Convert f32 [-1.0, 1.0] to i16
        let pcm: Vec<i16> = resampled.iter().map(|&s| {
            (s.clamp(-1.0, 1.0) * 32767.0) as i16
        }).collect();

        let flac_bytes = encoder::encode_flac(&pcm)?;
        Ok(encoder::audio_to_base64(&flac_bytes))
    }
}

/// Mix interleaved multi-channel samples to mono by averaging.
fn mix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels as usize;
    samples.chunks(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}

/// Resample using linear interpolation.
fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    (0..output_len).map(|i| {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input[idx];
        let b = if idx + 1 < input.len() { input[idx + 1] } else { a };
        a + (b - a) * frac
    }).collect()
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
        let mono = mix_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < 1e-6);
        assert!((mono[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_resample_downsample() {
        let input = vec![0.0, 0.5, 1.0, 0.5];
        let output = resample(&input, 48_000, 16_000);
        assert!(!output.is_empty());
        assert!(output.len() < input.len());
    }

    #[test]
    fn test_resample_same_rate() {
        let input = vec![0.1, 0.2, 0.3];
        let output = resample(&input, 16_000, 16_000);
        assert_eq!(output, input);
    }
}
