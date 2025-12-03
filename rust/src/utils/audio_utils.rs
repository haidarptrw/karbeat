// src/utils/audio_utils.rs

/// Robust Stereo Downsampling (Min-Max Binning).
///
/// ========================================================
/// 
/// Output Format per bin: [Left_Min, Left_Max, Right_Min, Right_Max]
/// 
/// ========================================================
/// 
/// Total output size = TARGET_BINS * 4
pub fn downsample(buffer: &[f32]) -> Vec<f32> {
    const TARGET_BINS: usize = 1000; // 1000 horizontal pixels
    const CHANNELS: usize = 2;       // We assume interleaved stereo input

    if buffer.is_empty() {
        return Vec::new();
    }

    let total_frames = buffer.len() / CHANNELS;
    
    // Safety: If buffer is tiny, just return it directly (padded if mono)
    if total_frames <= TARGET_BINS {
        let mut raw_out = Vec::with_capacity(buffer.len() * 2);
        for chunk in buffer.chunks(CHANNELS) {
            // Push L
            raw_out.push(chunk[0]); // Min
            raw_out.push(chunk[0]); // Max
            // Push R
            let r = if chunk.len() > 1 { chunk[1] } else { chunk[0] };
            raw_out.push(r); // Min
            raw_out.push(r); // Max
        }
        return raw_out;
    }

    let frames_per_bin = total_frames / TARGET_BINS;
    let mut out = Vec::with_capacity(TARGET_BINS * 4);

    for bin_idx in 0..TARGET_BINS {
        let start_frame = bin_idx * frames_per_bin;
        let end_frame = (start_frame + frames_per_bin).min(total_frames);

        // Track Min/Max for Left (0) and Right (1) separately
        let mut min_l = 1.0f32;
        let mut max_l = -1.0f32;
        let mut min_r = 1.0f32;
        let mut max_r = -1.0f32;
        
        // If bin is empty/skipped, reset to 0
        if start_frame >= end_frame {
             min_l = 0.0; max_l = 0.0; min_r = 0.0; max_r = 0.0;
        }

        let mut first = true;

        for i in start_frame..end_frame {
            let idx = i * CHANNELS;
            if idx + 1 >= buffer.len() { break; }

            let l = buffer[idx];
            let r = buffer[idx + 1];

            if first {
                min_l = l; max_l = l;
                min_r = r; max_r = r;
                first = false;
            } else {
                if l < min_l { min_l = l; }
                if l > max_l { max_l = l; }
                
                if r < min_r { min_r = r; }
                if r > max_r { max_r = r; }
            }
        }

        // Push 4 values
        out.push(min_l);
        out.push(max_l);
        out.push(min_r);
        out.push(max_r);
    }

    out
}