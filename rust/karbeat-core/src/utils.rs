use std::sync::Arc;

use memmap2::Mmap;

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