pub mod encoder;
pub mod input;
pub mod recorder;

use cpal::traits::{DeviceTrait, HostTrait};

/// Mix interleaved multi-channel samples to mono by averaging.
pub(crate) fn mix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = channels as usize;
    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Resample mono PCM using linear interpolation.
pub(crate) fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let fraction = (src_pos - idx as f64) as f32;
            let first = input[idx];
            let second = if idx + 1 < input.len() {
                input[idx + 1]
            } else {
                first
            };
            first + (second - first) * fraction
        })
        .collect()
}

/// Find an input device by name.
/// Returns the system default device for "system-default" or empty string.
/// Falls back to default device if the named device is not found.
pub fn find_input_device(device_name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();

    if device_name.is_empty() || device_name == "system-default" {
        return host.default_input_device();
    }

    // Try to find the named device
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_name {
                    return Some(device);
                }
            }
        }
    }

    // Fallback to default device
    eprintln!("Microphone '{}' not found, falling back to default", device_name);
    host.default_input_device()
}
