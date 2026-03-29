/// WSOLA (Waveform Similarity Overlap-Add) time-stretching.
///
/// Changes playback speed without altering pitch — the same algorithm family
/// used by SoundTouch, ffmpeg's atempo filter, and YouTube's speed controls.
///
/// How it works:
/// 1. Split audio into overlapping windows
/// 2. For each output window, find the best-matching position in the input
///    (within a search range) using cross-correlation
/// 3. Overlap-add the windows at the new spacing determined by the speed factor
///
/// This preserves pitch because each window contains the original waveform —
/// we're just rearranging how windows overlap in time.

/// Window size in samples. ~20ms at 24kHz gives good quality for speech.
const WINDOW_SIZE: usize = 512;

/// Search range: how far to look for the best overlap match.
const SEARCH_RANGE: usize = 128;

/// Time-stretch audio samples by the given speed factor.
/// speed > 1.0 = faster (shorter output), speed < 1.0 = slower (longer output).
pub fn wsola(samples: &[f32], speed: f32) -> Vec<f32> {
    if samples.len() < WINDOW_SIZE * 2 || (speed - 1.0).abs() < 0.01 {
        return samples.to_vec();
    }

    let hop_in = (WINDOW_SIZE as f32 * speed) as usize;
    let hop_out = WINDOW_SIZE / 2;
    let output_len = ((samples.len() as f32) / speed) as usize;
    let mut output = vec![0.0f32; output_len + WINDOW_SIZE];
    let mut norm = vec![0.0f32; output_len + WINDOW_SIZE];

    // Hann window for smooth overlap-add
    let window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| {
            let t = i as f32 / (WINDOW_SIZE - 1) as f32;
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
        })
        .collect();

    let mut input_pos: usize = 0;
    let mut output_pos: usize = 0;

    while input_pos + WINDOW_SIZE < samples.len() && output_pos + WINDOW_SIZE < output.len() {
        // Find the best offset within search range using cross-correlation
        let best_offset = if output_pos == 0 {
            0 // First window: no search needed
        } else {
            find_best_offset(samples, input_pos, &output, output_pos, &window)
        };

        let src_start = (input_pos as isize + best_offset as isize)
            .max(0) as usize;

        if src_start + WINDOW_SIZE > samples.len() {
            break;
        }

        // Overlap-add with Hann window
        for i in 0..WINDOW_SIZE {
            let w = window[i];
            output[output_pos + i] += samples[src_start + i] * w;
            norm[output_pos + i] += w;
        }

        input_pos += hop_in;
        output_pos += hop_out;
    }

    // Normalize overlapping regions
    for i in 0..output.len() {
        if norm[i] > 1e-6 {
            output[i] /= norm[i];
        }
    }

    // Trim to actual content
    let actual_len = output_pos + WINDOW_SIZE;
    output.truncate(actual_len.min(output.len()));

    // Remove trailing silence
    while output.last().is_some_and(|&s| s.abs() < 1e-6) {
        output.pop();
    }

    output
}

/// Find the best offset (within SEARCH_RANGE) for overlapping the next window.
/// Uses normalized cross-correlation between the overlap region in the output
/// and candidate positions in the input.
fn find_best_offset(
    input: &[f32],
    input_pos: usize,
    output: &[f32],
    output_pos: usize,
    window: &[f32],
) -> isize {
    let half = SEARCH_RANGE / 2;
    let mut best_offset: isize = 0;
    let mut best_corr: f32 = f32::NEG_INFINITY;

    // Compare the overlap region of the output with candidate input positions
    let overlap = WINDOW_SIZE / 4; // Only compare first quarter for speed

    for delta in 0..SEARCH_RANGE {
        let offset = delta as isize - half as isize;
        let candidate = (input_pos as isize + offset).max(0) as usize;

        if candidate + overlap > input.len() || output_pos + overlap > output.len() {
            continue;
        }

        // Cross-correlation
        let mut corr: f32 = 0.0;
        let mut energy_a: f32 = 0.0;
        let mut energy_b: f32 = 0.0;

        for i in 0..overlap {
            let a = output[output_pos + i] * window[i];
            let b = input[candidate + i] * window[i];
            corr += a * b;
            energy_a += a * a;
            energy_b += b * b;
        }

        let norm = (energy_a * energy_b).sqrt();
        let normalized = if norm > 1e-8 { corr / norm } else { 0.0 };

        if normalized > best_corr {
            best_corr = normalized;
            best_offset = offset;
        }
    }

    best_offset
}
