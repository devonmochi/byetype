use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

use super::encoder;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
}

struct ActiveRecording {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<i16>>>,
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

    pub fn start(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state == RecordingState::Recording {
            return Err("Already recording".to_string());
        }

        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| "No input device available".to_string())?;

        let desired_config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16_000),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        let stream = device.build_input_stream(
            &desired_config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mut buf = samples_clone.lock().unwrap();
                buf.extend_from_slice(data);
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        ).map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

        *state = RecordingState::Recording;
        let mut active = self.active.lock().unwrap();
        *active = Some(ActiveRecording { stream, samples });

        Ok(())
    }

    pub fn stop(&self) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        if *state != RecordingState::Recording {
            return Err("Not recording".to_string());
        }

        let mut active_guard = self.active.lock().unwrap();
        let recording = active_guard.take()
            .ok_or_else(|| "No active recording".to_string())?;

        drop(recording.stream);

        let samples = recording.samples.lock().unwrap();
        if samples.is_empty() {
            *state = RecordingState::Idle;
            return Err("No audio data captured".to_string());
        }

        let wav_bytes = encoder::encode_wav(&samples)?;
        let base64_audio = encoder::wav_to_base64(&wav_bytes);

        *state = RecordingState::Idle;
        Ok(base64_audio)
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
}
