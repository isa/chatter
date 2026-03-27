use std::path::Path;

use mp3lame_encoder::{Builder, FlushNoGap, MonoPcm};

pub mod playback;

/// Convert float32 samples ([-1.0, 1.0]) to i16 PCM.
pub fn samples_f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

/// Encode i16 PCM samples to MP3 and write to file.
/// sample_rate is typically 24000 for Qwen3-TTS output.
pub fn encode_wav_to_mp3(
    samples: &[i16],
    sample_rate: u32,
    output_path: &Path,
) -> anyhow::Result<()> {
    let mut builder = Builder::new().expect("valid builder");
    builder.set_num_channels(1).expect("valid channels");
    builder
        .set_sample_rate(sample_rate)
        .expect("valid sample rate");
    builder
        .set_brate(mp3lame_encoder::Bitrate::Kbps192)
        .expect("valid bitrate");
    builder
        .set_quality(mp3lame_encoder::Quality::Best)
        .expect("valid quality");
    let mut encoder = builder.build().expect("valid encoder");

    let input = MonoPcm(samples);
    let mut output = Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(samples.len()));
    encoder
        .encode_to_vec(input, &mut output)
        .map_err(|e| anyhow::anyhow!("MP3 encode failed: {e:?}"))?;
    encoder
        .flush_to_vec::<FlushNoGap>(&mut output)
        .map_err(|e| anyhow::anyhow!("MP3 flush failed: {e:?}"))?;

    std::fs::write(output_path, &output)?;
    Ok(())
}
