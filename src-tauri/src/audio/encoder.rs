use hound::{WavSpec, WavWriter};
use std::io::Cursor;

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;

pub fn wav_spec() -> WavSpec {
    WavSpec {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: hound::SampleFormat::Int,
    }
}

/// Encode PCM i16 samples into a WAV byte buffer.
pub fn encode_wav(samples: &[i16]) -> Result<Vec<u8>, String> {
    let spec = wav_spec();
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;
        for &sample in samples {
            writer.write_sample(sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }
        writer.finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
    }
    Ok(cursor.into_inner())
}

/// Encode WAV bytes to Base64 string.
pub fn wav_to_base64(wav_bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(wav_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_produces_valid_header() {
        let samples = vec![0i16; 16_000];
        let wav = encode_wav(&samples).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 16_000 * 2);
    }

    #[test]
    fn test_wav_to_base64_roundtrip() {
        let samples = vec![100i16; 100];
        let wav = encode_wav(&samples).unwrap();
        let b64 = wav_to_base64(&wav);
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&b64).unwrap();
        assert_eq!(decoded, wav);
    }
}
