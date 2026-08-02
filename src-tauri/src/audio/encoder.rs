const SAMPLE_RATE: u32 = 16_000;

/// Encode PCM i16 samples into a FLAC byte buffer.
/// Lossless compression, typically ~50% smaller than WAV.
pub fn encode_flac(samples: &[i16]) -> Result<Vec<u8>, String> {
    encode_flac_inner(samples, None)
}

pub fn encode_flac_with_cancel(
    samples: &[i16],
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<Vec<u8>, String> {
    encode_flac_inner(samples, Some(cancellation))
}

fn encode_flac_inner(
    samples: &[i16],
    cancellation: Option<tokio_util::sync::CancellationToken>,
) -> Result<Vec<u8>, String> {
    use flacenc::bitsink::{BitSink, Bits, ByteSink};
    use flacenc::component::BitRepr;
    use flacenc::config;
    use flacenc::error::{SourceError, Verify};
    use flacenc::source::{Fill, MemSource, Source};

    struct CancellableSource {
        source: MemSource,
        cancellation: Option<tokio_util::sync::CancellationToken>,
    }

    struct CancellableSink {
        sink: ByteSink,
        cancellation: Option<tokio_util::sync::CancellationToken>,
    }

    impl CancellableSink {
        fn ensure_active(&self) -> Result<(), std::io::Error> {
            if self
                .cancellation
                .as_ref()
                .is_some_and(|token| token.is_cancelled())
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "request cancelled",
                ))
            } else {
                Ok(())
            }
        }
    }

    impl BitSink for CancellableSink {
        type Error = std::io::Error;

        fn align_to_byte(&mut self) -> Result<usize, Self::Error> {
            self.ensure_active()?;
            Ok(self.sink.align_to_byte().unwrap())
        }

        fn write_lsbs<T: Bits>(&mut self, value: T, bits: usize) -> Result<(), Self::Error> {
            self.ensure_active()?;
            Ok(self.sink.write_lsbs(value, bits).unwrap())
        }

        fn write_msbs<T: Bits>(&mut self, value: T, bits: usize) -> Result<(), Self::Error> {
            self.ensure_active()?;
            Ok(self.sink.write_msbs(value, bits).unwrap())
        }

        fn write<T: Bits>(&mut self, value: T) -> Result<(), Self::Error> {
            self.ensure_active()?;
            Ok(self.sink.write(value).unwrap())
        }
    }

    impl Source for CancellableSource {
        fn channels(&self) -> usize {
            self.source.channels()
        }

        fn bits_per_sample(&self) -> usize {
            self.source.bits_per_sample()
        }

        fn sample_rate(&self) -> usize {
            self.source.sample_rate()
        }

        fn read_samples<F: Fill>(
            &mut self,
            block_size: usize,
            dest: &mut F,
        ) -> Result<usize, SourceError> {
            if self
                .cancellation
                .as_ref()
                .is_some_and(|token| token.is_cancelled())
            {
                return Err(SourceError::from_io_error(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "request cancelled",
                )));
            }
            self.source.read_samples(block_size, dest)
        }

        fn len_hint(&self) -> Option<usize> {
            self.source.len_hint()
        }
    }

    let mut samples_i32 = Vec::with_capacity(samples.len());
    for chunk in samples.chunks(4096) {
        if cancellation
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            return Err("请求已取消".to_string());
        }
        samples_i32.extend(chunk.iter().map(|&sample| sample as i32));
    }
    let source = CancellableSource {
        source: MemSource::from_samples(&samples_i32, 1, 16, SAMPLE_RATE as usize),
        cancellation: cancellation.clone(),
    };
    let encoder_config = config::Encoder::default()
        .into_verified()
        .map_err(|e| format!("FLAC config error: {:?}", e))?;
    let flac_stream =
        flacenc::encode_with_fixed_block_size(&encoder_config, source, encoder_config.block_size)
            .map_err(|error| {
            if cancellation
                .as_ref()
                .is_some_and(|token| token.is_cancelled())
            {
                "请求已取消".to_string()
            } else {
                format!("FLAC encode error: {:?}", error)
            }
        })?;

    if cancellation
        .as_ref()
        .is_some_and(|token| token.is_cancelled())
    {
        return Err("请求已取消".to_string());
    }

    let mut sink = CancellableSink {
        sink: ByteSink::new(),
        cancellation: cancellation.clone(),
    };
    flac_stream.write(&mut sink).map_err(|error| {
        if cancellation
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            "请求已取消".to_string()
        } else {
            format!("FLAC write error: {:?}", error)
        }
    })?;
    Ok(sink.sink.into_inner())
}

/// Encode bytes to Base64 string.
pub fn audio_to_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
