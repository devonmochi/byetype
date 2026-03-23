pub mod encoder;
pub mod recorder;

use cpal::traits::{DeviceTrait, HostTrait};

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
