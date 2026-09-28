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

/// 把样本补零到整块长度的整数倍，保证编码器每次都能读满一块。
fn pad_to_block_boundary(samples: &mut Vec<i32>, block_size: usize) {
    if block_size == 0 {
        return;
    }
    let remainder = samples.len() % block_size;
    if remainder != 0 {
        samples.resize(samples.len() + block_size - remainder, 0);
    }
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
    let encoder_config = config::Encoder::default()
        .into_verified()
        .map_err(|e| format!("FLAC config error: {:?}", e))?;

    // flacenc 的定长分块编码把最后一帧按整块写出，块内没读到的位置保留复用缓冲里的
    // 旧样本，等于在音频末尾接了一小段前面录到的声音，转写结果结尾会多出一两个字。
    // 先把样本补零到整块长度，最后一帧就是静音。
    let block_size = encoder_config.block_size;
    pad_to_block_boundary(&mut samples_i32, block_size);

    let source = CancellableSource {
        source: MemSource::from_samples(&samples_i32, 1, 16, SAMPLE_RATE as usize),
        cancellation: cancellation.clone(),
    };
    let flac_stream =
        flacenc::encode_with_fixed_block_size(&encoder_config, source, block_size)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 编码器默认的定长块大小，测试跟着配置走，避免两边写死不一样。
    fn block_size() -> usize {
        flacenc::config::Encoder::default().block_size
    }

    /// 读取 FLAC STREAMINFO 里的 total_samples 字段。
    fn streaminfo_total_samples(flac: &[u8]) -> usize {
        assert_eq!(&flac[..4], b"fLaC");
        let info = &flac[8..42];
        let packed = u64::from_be_bytes(info[10..18].try_into().unwrap());
        (packed & ((1u64 << 36) - 1)) as usize
    }

    #[test]
    fn pads_samples_to_block_boundary_with_silence() {
        let real_len = block_size() + 3;
        let mut samples = vec![7i32; real_len];

        pad_to_block_boundary(&mut samples, block_size());

        assert_eq!(samples.len(), block_size() * 2);
        assert_eq!(samples[real_len - 1], 7);
        assert!(samples[real_len..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn leaves_whole_block_length_untouched() {
        let mut samples = vec![7i32; block_size() * 2];

        pad_to_block_boundary(&mut samples, block_size());

        assert_eq!(samples.len(), block_size() * 2);
    }

    /// 编码后的音频长度必须是整块长度，否则最后一帧会夹带上一块遗留的样本，
    /// 转写结果结尾多出的一两个字就来自这段声音。
    #[test]
    fn encoded_stream_covers_whole_blocks_only() {
        let flac = encode_flac(&vec![1200i16; block_size() * 2 + 123]).expect("encode");

        let total = streaminfo_total_samples(&flac);
        assert_eq!(total, block_size() * 3);
        assert_eq!(total % block_size(), 0);
    }

    #[test]
    fn encodes_short_clip_into_one_block() {
        let flac = encode_flac(&vec![300i16; 1600]).expect("encode");

        let expected = 1600usize.div_ceil(block_size()) * block_size();
        assert_eq!(streaminfo_total_samples(&flac), expected);
    }
}
