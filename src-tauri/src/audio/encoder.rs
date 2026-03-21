const SAMPLE_RATE: u32 = 16_000;

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
