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

/// Encode PCM i16 samples into an OGG Opus byte buffer.
/// Achieves ~10x compression compared to WAV for speech audio.
pub fn encode_ogg_opus(pcm_i16: &[i16]) -> Result<Vec<u8>, String> {
    use audiopus::coder::Encoder as OpusEncoder;
    use audiopus::{Application, Channels as OpusChannels, SampleRate as OpusSampleRate, Bitrate};
    use ogg::writing::{PacketWriter, PacketWriteEndInfo};

    const FRAME_SIZE: usize = 320; // 20ms at 16kHz

    let mut encoder = OpusEncoder::new(
        OpusSampleRate::Hz16000,
        OpusChannels::Mono,
        Application::Voip,
    ).map_err(|e| format!("Opus encoder error: {:?}", e))?;

    encoder.set_bitrate(Bitrate::BitsPerSecond(24000))
        .map_err(|e| format!("Opus set bitrate error: {:?}", e))?;

    let mut output = Cursor::new(Vec::new());
    let serial: u32 = 1;

    {
        let mut writer = PacketWriter::new(&mut output);

        // OpusHead header (RFC 7845)
        let mut head = Vec::with_capacity(19);
        head.extend_from_slice(b"OpusHead");
        head.push(1); // version
        head.push(1); // channel count
        head.extend_from_slice(&0u16.to_le_bytes()); // pre-skip
        head.extend_from_slice(&SAMPLE_RATE.to_le_bytes()); // input sample rate
        head.extend_from_slice(&0i16.to_le_bytes()); // output gain
        head.push(0); // channel mapping family

        writer.write_packet(head, serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| format!("OGG write error: {:?}", e))?;

        // OpusTags header
        let vendor = b"byetype";
        let mut tags = Vec::with_capacity(8 + 4 + vendor.len() + 4);
        tags.extend_from_slice(b"OpusTags");
        tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags.extend_from_slice(vendor);
        tags.extend_from_slice(&0u32.to_le_bytes()); // no user comments

        writer.write_packet(tags, serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| format!("OGG write error: {:?}", e))?;

        // Encode audio frames
        let mut opus_buf = vec![0u8; 4000];
        let frame_count = if pcm_i16.is_empty() { 0 } else {
            (pcm_i16.len() + FRAME_SIZE - 1) / FRAME_SIZE
        };
        let mut granule: u64 = 0;

        for i in 0..frame_count {
            let start = i * FRAME_SIZE;
            let end = (start + FRAME_SIZE).min(pcm_i16.len());

            // Pad last frame with silence if needed
            let frame: Vec<i16> = if end - start < FRAME_SIZE {
                let mut f = pcm_i16[start..end].to_vec();
                f.resize(FRAME_SIZE, 0);
                f
            } else {
                pcm_i16[start..end].to_vec()
            };

            let len = encoder.encode(&frame, &mut opus_buf)
                .map_err(|e| format!("Opus encode error: {:?}", e))?;

            granule += FRAME_SIZE as u64;
            let end_info = if i == frame_count - 1 {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::NormalPacket
            };

            writer.write_packet(
                opus_buf[..len].to_vec(),
                serial,
                end_info,
                granule,
            ).map_err(|e| format!("OGG write error: {:?}", e))?;
        }
    }

    Ok(output.into_inner())
}

/// Encode PCM i16 samples into a FLAC byte buffer.
/// Lossless compression, typically ~50% smaller than WAV.
pub fn encode_flac(samples: &[i16]) -> Result<Vec<u8>, String> {
    use flacenc::bitsink::ByteSink;
    use flacenc::component::BitRepr;
    use flacenc::config;
    use flacenc::error::Verify;
    use flacenc::source::MemSource;

    let samples_i32: Vec<i32> = samples.iter().map(|&s| s as i32).collect();
    let source = MemSource::from_samples(&samples_i32, 1, 16, SAMPLE_RATE as usize);
    let encoder_config = config::Encoder::default()
        .into_verified()
        .map_err(|e| format!("FLAC config error: {:?}", e))?;
    let flac_stream = flacenc::encode_with_fixed_block_size(
        &encoder_config,
        source,
        encoder_config.block_size,
    )
    .map_err(|e| format!("FLAC encode error: {:?}", e))?;

    let mut sink = ByteSink::new();
    flac_stream.write(&mut sink)
        .map_err(|e| format!("FLAC write error: {:?}", e))?;
    Ok(sink.as_slice().to_vec())
}

/// Encode bytes to Base64 string.
pub fn audio_to_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Encode WAV bytes to Base64 string (kept for compatibility).
pub fn wav_to_base64(wav_bytes: &[u8]) -> String {
    audio_to_base64(wav_bytes)
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
