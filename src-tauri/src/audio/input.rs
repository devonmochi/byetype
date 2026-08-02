use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::{encoder, mix_to_mono, resample};

const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug)]
pub struct NormalizedAudio {
    pub flac: Vec<u8>,
}

pub fn normalize_audio(
    bytes: &[u8],
    content_type: &str,
    max_duration_seconds: u32,
) -> Result<NormalizedAudio, String> {
    let extension = extension_for_content_type(content_type)
        .ok_or_else(|| format!("不支持的音频格式：{}", content_type))?;
    if bytes.is_empty() {
        return Err("音频内容为空".to_string());
    }

    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
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
        sample_rate.get_or_insert(spec.rate);
        channels.get_or_insert(spec.channels.count() as u16);
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        interleaved.extend_from_slice(samples.samples());
    }

    let sample_rate = sample_rate.ok_or_else(|| "音频中没有可解码内容".to_string())?;
    let channels = channels.ok_or_else(|| "音频中没有声道信息".to_string())?;
    let mono = if channels > 1 {
        mix_to_mono(&interleaved, channels)
    } else {
        interleaved
    };
    let duration_seconds = mono.len() as f64 / sample_rate as f64;
    if duration_seconds > max_duration_seconds as f64 {
        return Err(format!("音频时长超过 {} 秒限制", max_duration_seconds));
    }

    let resampled = resample(&mono, sample_rate, TARGET_SAMPLE_RATE);
    let pcm: Vec<i16> = resampled
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();

    Ok(NormalizedAudio {
        flac: encoder::encode_flac(&pcm)?,
    })
}

fn extension_for_content_type(content_type: &str) -> Option<&'static str> {
    match content_type
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "audio/flac" | "audio/x-flac" => Some("flac"),
        "audio/mpeg" | "audio/mp3" => Some("mp3"),
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" => Some("m4a"),
        "audio/wav" | "audio/wave" | "audio/x-wav" => Some("wav"),
        _ => None,
    }
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

        let normalized = normalize_audio(&wav, "audio/wav", 180).unwrap();

        assert_eq!(flac_sample_rate_and_channels(&normalized.flac), (16_000, 1));
    }

    #[test]
    fn unsupported_content_type_is_rejected() {
        let error = normalize_audio(b"not audio", "text/plain", 180).unwrap_err();

        assert_eq!(error, "不支持的音频格式：text/plain");
    }

    #[test]
    fn audio_over_the_configured_duration_is_rejected() {
        let wav = mono_wav(8_000, &[0; 8_000]);

        let error = normalize_audio(&wav, "audio/wav", 0).unwrap_err();

        assert_eq!(error, "音频时长超过 0 秒限制");
    }
}
