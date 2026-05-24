use flutter_rust_bridge::frb;
pub use karbeat_plugin_api::types::{BufferDataType, ZeroCopyBuffer};

#[frb(opaque)]
#[derive(Clone)]
pub struct ZeroCopyHandle {
    buffer: ZeroCopyBuffer,
}
impl ZeroCopyHandle {
    /// Tells Dart what kind of TypedList to create
    #[frb(sync)]
    pub fn data_type(&self) -> BufferDataType {
        match &self.buffer {
            ZeroCopyBuffer::Float32(_) => BufferDataType::Float32,
            ZeroCopyBuffer::Uint8(_) => BufferDataType::Uint8,
            ZeroCopyBuffer::Int32(_) => BufferDataType::Int32,
            ZeroCopyBuffer::Int8(_) => BufferDataType::Int8,
        }
    }
    /// Gets the raw memory address
    #[frb(sync)]
    pub fn memory_address(&self) -> usize {
        match &self.buffer {
            ZeroCopyBuffer::Float32(b) => b.as_ptr() as usize,
            ZeroCopyBuffer::Uint8(b) => b.as_ptr() as usize,
            ZeroCopyBuffer::Int32(b) => b.as_ptr() as usize,
            ZeroCopyBuffer::Int8(b) => b.as_ptr() as usize,
        }
    }
    /// Gets the number of elements
    #[frb(sync)]
    pub fn length_elements(&self) -> usize {
        match &self.buffer {
            ZeroCopyBuffer::Float32(b) => b.len(),
            ZeroCopyBuffer::Uint8(b) => b.len(),
            ZeroCopyBuffer::Int32(b) => b.len(),
            ZeroCopyBuffer::Int8(b) => b.len(),
        }
    }

    #[frb(sync)]
    pub fn new(buffer: ZeroCopyBuffer) -> Self {
        Self { buffer }
    }
}
