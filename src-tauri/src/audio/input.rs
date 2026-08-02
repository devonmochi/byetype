use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio_util::sync::CancellationToken;

use super::encoder;

const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug)]
pub struct NormalizedAudio {
    pub flac: Vec<u8>,
}

pub fn normalize_audio(
    bytes: Vec<u8>,
    content_type: &str,
    max_duration_seconds: u32,
    cancellation: &CancellationToken,
) -> Result<NormalizedAudio, String> {
    let extension = validate_content_type(content_type)?;
    if bytes.is_empty() {
        return Err("音频内容为空".to_string());
    }
    ensure_not_cancelled(cancellation)?;

    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(extension);
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| format!("无法读取音频：{}", error))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "音频中没有可用轨道".to_string())?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| format!("无法创建音频解码器：{}", error))?;

    let mut interleaved = Vec::<f32>::new();
    let mut sample_rate = None;
    let mut channels = None;

    loop {
        ensure_not_cancelled(cancellation)?;
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err("音频参数在文件中发生变化，暂不支持".to_string());
            }
            Err(error) => return Err(format!("读取音频数据失败：{}", error)),
        };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(error) => return Err(format!("解码音频失败：{}", error)),
        };
        let spec = *decoded.spec();
        let current_sample_rate = *sample_rate.get_or_insert(spec.rate);
        let current_channels = *channels.get_or_insert(spec.channels.count() as u16);
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        interleaved.extend_from_slice(samples.samples());
        let max_interleaved_samples =
            max_duration_seconds as u64 * current_sample_rate as u64 * current_channels as u64;
        if interleaved.len() as u64 > max_interleaved_samples {
            return Err(format!("音频时长超过{}秒限制", max_duration_seconds));
        }
    }

    ensure_not_cancelled(cancellation)?;
    let sample_rate = sample_rate.ok_or_else(|| "音频中没有可解码内容".to_string())?;
    let channels = channels.ok_or_else(|| "音频中没有声道信息".to_string())?;
    let mono = mix_to_mono_with_cancel(interleaved, channels, cancellation)?;
    let duration_seconds = mono.len() as f64 / sample_rate as f64;
    if duration_seconds > max_duration_seconds as f64 {
        return Err(format!("音频时长超过{}秒限制", max_duration_seconds));
    }

    let resampled = resample_with_cancel(&mono, sample_rate, cancellation)?;
    let mut pcm = Vec::with_capacity(resampled.len());
    for samples in resampled.chunks(4096) {
        ensure_not_cancelled(cancellation)?;
        pcm.extend(
            samples
                .iter()
                .map(|sample| (sample.clamp(-1.0, 1.0) * 32767.0) as i16),
        );
    }

    Ok(NormalizedAudio {
        flac: encoder::encode_flac_with_cancel(&pcm, cancellation.clone())?,
    })
}

fn mix_to_mono_with_cancel(
    interleaved: Vec<f32>,
    channels: u16,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, String> {
    if channels == 1 {
        return Ok(interleaved);
    }
    let channel_count = channels as usize;
    let mut mono = Vec::with_capacity(interleaved.len() / channel_count);
    for frames in interleaved.chunks(channel_count * 4096) {
        ensure_not_cancelled(cancellation)?;
        mono.extend(
            frames
                .chunks(channel_count)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32),
        );
    }
    Ok(mono)
}

fn resample_with_cancel(
    input: &[f32],
    from_rate: u32,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if from_rate == TARGET_SAMPLE_RATE {
        let mut output = Vec::with_capacity(input.len());
        for samples in input.chunks(4096) {
            ensure_not_cancelled(cancellation)?;
            output.extend_from_slice(samples);
        }
        return Ok(output);
    }
    let ratio = from_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for start in (0..output_len).step_by(4096) {
        ensure_not_cancelled(cancellation)?;
        let end = (start + 4096).min(output_len);
        output.extend((start..end).map(|index| {
            let source_position = index as f64 * ratio;
            let source_index = source_position as usize;
            let fraction = (source_position - source_index as f64) as f32;
            let first = input[source_index];
            let second = input.get(source_index + 1).copied().unwrap_or(first);
            first + (second - first) * fraction
        }));
    }
    Ok(output)
}

fn ensure_not_cancelled(cancellation: &CancellationToken) -> Result<(), String> {
    if cancellation.is_cancelled() {
        Err("请求已取消".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn validate_content_type(content_type: &str) -> Result<&'static str, String> {
    let extension = match content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "audio/flac" | "audio/x-flac" => Some("flac"),
        "audio/mpeg" | "audio/mp3" => Some("mp3"),
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" => Some("m4a"),
        "audio/wav" | "audio/wave" | "audio/x-wav" => Some("wav"),
        _ => None,
    };
    extension.ok_or_else(|| format!("不支持的音频格式：{}", content_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    fn flac_sample_rate_and_channels(flac: &[u8]) -> (u32, u16) {
        assert!(flac.starts_with(b"fLaC"));
        let packed = u64::from_be_bytes(flac[18..26].try_into().unwrap());
        let sample_rate = (packed >> 44) as u32;
        let channels = (((packed >> 41) & 0b111) + 1) as u16;
        (sample_rate, channels)
    }

    #[test]
    fn wav_input_is_normalized_to_flac() {
        let wav = mono_wav(8_000, &[0, 1_000, -1_000, 0]);

        let normalized = normalize_audio(wav, "audio/wav", 180, &CancellationToken::new()).unwrap();

        assert_eq!(flac_sample_rate_and_channels(&normalized.flac), (16_000, 1));
    }

    #[test]
    fn unsupported_content_type_is_rejected() {
        let error = normalize_audio(
            b"not audio".to_vec(),
            "text/plain",
            180,
            &CancellationToken::new(),
        )
        .unwrap_err();

        assert_eq!(error, "不支持的音频格式：text/plain");
    }

    #[test]
    fn audio_over_the_configured_duration_is_rejected() {
        let wav = mono_wav(8_000, &[0; 8_000]);

        let error = normalize_audio(wav, "audio/wav", 0, &CancellationToken::new()).unwrap_err();

        assert_eq!(error, "音频时长超过0秒限制");
    }

    #[test]
    fn cancelled_audio_conversion_stops_before_decoding() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let error = normalize_audio(
            mono_wav(8_000, &[0; 8_000]),
            "audio/wav",
            180,
            &cancellation,
        )
        .unwrap_err();

        assert_eq!(error, "请求已取消");
    }
}
