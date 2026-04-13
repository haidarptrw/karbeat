use dasp::slice;

/// Robust Stereo Downsampling (Min-Max Binning) with Dasp.
/// Supports generic Mono (1) or Stereo (2) channel counts safely.
/// 
/// Output Format per bin: [Left_Min, Left_Max, Right_Min, Right_Max]
pub fn downsample(buffer: &[f32], channels: usize, target_bins: usize) -> Vec<f32> {
    if buffer.is_empty() || channels == 0 || target_bins == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(target_bins * 4);

    if channels == 1 {
        // DASP: Safely cast flat slice to Mono frames without allocation
        let frames: &[[f32; 1]] = slice::from_sample_slice(buffer).unwrap_or(&[]);
        let total_frames = frames.len();
        
        // Handle tiny buffers by padding directly to the output
        if total_frames <= target_bins {
            for frame in frames {
                let val = frame[0];
                out.push(val); out.push(val); // L min/max
                out.push(val); out.push(val); // R min/max
            }
            return out;
        }

        let frames_per_bin = total_frames / target_bins;

        for bin_idx in 0..target_bins {
            let start_frame = bin_idx * frames_per_bin;
            // Ensure the very last bin absorbs any remainder frames
            let end_frame = if bin_idx == target_bins - 1 { total_frames } else { start_frame + frames_per_bin };

            let mut min_val = 1.0f32;
            let mut max_val = -1.0f32;

            for i in start_frame..end_frame {
                let val = frames[i][0];
                if val < min_val { min_val = val; }
                if val > max_val { max_val = val; }
            }
            
            if min_val > max_val { min_val = 0.0; max_val = 0.0; }

            // Duplicate mono signal to Left and Right
            out.push(min_val); out.push(max_val); 
            out.push(min_val); out.push(max_val); 
        }
    } else {
        // DASP: Safely cast flat slice to Stereo frames
        let frames: &[[f32; 2]] = slice::from_sample_slice(buffer).unwrap_or(&[]);
        let total_frames = frames.len();

        if total_frames <= target_bins {
            for frame in frames {
                out.push(frame[0]); out.push(frame[0]);
                out.push(frame[1]); out.push(frame[1]);
            }
            return out;
        }

        let frames_per_bin = total_frames / target_bins;

        for bin_idx in 0..target_bins {
            let start_frame = bin_idx * frames_per_bin;
            let end_frame = if bin_idx == target_bins - 1 { total_frames } else { start_frame + frames_per_bin };

            let mut min_l = 1.0f32; let mut max_l = -1.0f32;
            let mut min_r = 1.0f32; let mut max_r = -1.0f32;

            for i in start_frame..end_frame {
                let l = frames[i][0];
                let r = frames[i][1];
                
                if l < min_l { min_l = l; }
                if l > max_l { max_l = l; }
                if r < min_r { min_r = r; }
                if r > max_r { max_r = r; }
            }

            if min_l > max_l {
                min_l = 0.0; max_l = 0.0; min_r = 0.0; max_r = 0.0;
            }

            out.push(min_l); out.push(max_l);
            out.push(min_r); out.push(max_r);
        }
    }

    out
}

/// Quantize f32 [-1.0, 1.0] → i8 [-127, 127]
pub fn quantize_to_i8(input: &[f32]) -> Vec<i8> {
    input
        .iter()
        .map(|&v| {
            let clamped = v.clamp(-1.0, 1.0);
            (clamped * 127.0) as i8
        })
        .collect()
}

/// Downsample using max absolute value (preserves peaks)
pub fn downsample_max_abs(input: &[i8], chunk_size: usize) -> Vec<i8> {
    input
        .chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .copied()
                .max_by_key(|v| v.abs())
                .unwrap_or(0)
        })
        .collect()
}


/// Create mip maps for waveform buffer
/// This includes 1 sample per bin, 4 samples per bin, 16 samples per bin, 
/// 64 samples per bin, 256 samples per bin, 1024 samples per bin, each sample/bin
/// mapped as HashMap key
pub fn setup_mipmaps(buffer: &[f32], channels: usize) -> hashbrown::HashMap<u32, Vec<i8>> {
    let mut mipmaps = hashbrown::HashMap::new();

    if buffer.is_empty() || channels == 0 {
        return mipmaps;
    }

    let total_frames = buffer.len() / channels;

    // Mipmap levels (samples per bin)
    let levels = [1, 4, 16, 64, 256, 1024];

    for &samples_per_bin in &levels {
        // Compute number of bins for this mip level
        let target_bins = (total_frames / samples_per_bin).max(1);

        // Step 1: downsample (min/max binning, f32)
        let downsampled_f32 = downsample(buffer, channels, target_bins);

        // Step 2: quantize → i8
        let quantized = quantize_to_i8(&downsampled_f32);

        mipmaps.insert(samples_per_bin as u32, quantized);
    }

    mipmaps
}

pub fn find_best_mipmap(
    mipmaps: &hashbrown::HashMap<u32, Vec<i8>>,
    target: u32,
) -> Option<&Vec<i8>> {
    mipmaps
        .iter()
        .filter(|(k, _)| **k >= target)
        .min_by_key(|(k, _)| *k)
        .or_else(|| {
            // fallback to largest available
            mipmaps.iter().max_by_key(|(k, _)| *k)
        })
        .map(|(_, v)| v)
}