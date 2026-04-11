use std::sync::Arc;

use memmap2::Mmap;
use wide::f32x4;

#[inline]
pub fn get_waveform_buffer(buffer: &Option<Arc<Mmap>>) -> Option<&[f32]> {
    // 1. Map the Option<&Arc<Mmap>> to Option<&Mmap>
    let buffer_mmap = buffer.as_deref()?;
    
    // 2. Get the byte slice from the Mmap
    let bytes = &buffer_mmap[..];
    
    // 3. Cast the slice. 
    // The compiler automatically ties the output lifetime to the input 'buffer'.
    Some(bytemuck::cast_slice(bytes))
}

/// Adds `src` buffer to `dest` buffer using SIMD instructions.
#[inline(always)]
pub fn apply_simd_mix(dest: &mut [f32], src: &[f32]) {
    let mut dest_iter = dest.chunks_exact_mut(4);
    let mut src_iter = src.chunks_exact(4);
    
    for (d, s) in dest_iter.by_ref().zip(src_iter.by_ref()) {
        let mut d_v = f32x4::new([d[0], d[1], d[2], d[3]]);
        let s_v = f32x4::new([s[0], s[1], s[2], s[3]]);
        d_v += s_v;
        d.copy_from_slice(&d_v.to_array());
    }
    
    // Process remaining samples (0 to 3)
    for (d, s) in dest_iter.into_remainder().iter_mut().zip(src_iter.remainder()) {
        *d += *s;
    }
}

/// Adds `src` buffer multiplied by `gain` to `dest` buffer using SIMD FMA.
#[inline(always)]
pub fn apply_simd_mix_gain(dest: &mut [f32], src: &[f32], gain: f32) {
    let gain_v = f32x4::splat(gain);
    let mut dest_iter = dest.chunks_exact_mut(4);
    let mut src_iter = src.chunks_exact(4);
    
    for (d, s) in dest_iter.by_ref().zip(src_iter.by_ref()) {
        let mut d_v = f32x4::new([d[0], d[1], d[2], d[3]]);
        let s_v = f32x4::new([s[0], s[1], s[2], s[3]]);
        d_v += s_v * gain_v;
        d.copy_from_slice(&d_v.to_array());
    }
    
    for (d, s) in dest_iter.into_remainder().iter_mut().zip(src_iter.remainder()) {
        *d += *s * gain;
    }
}