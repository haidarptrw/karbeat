// karbeat_utils::hash

/// Offset for FNV1a hash
pub const FNV_OFFSET: u32 = 2166136261;
const FNV_PRIME: u32 = 16777619;

/// Hash a string from scratch (used by UI or root nodes)
pub const fn hash_str(s: &str) -> u32 {
    hash_str_from(FNV_OFFSET, s)
}

/// Resume a hash from a previous prefix state
pub const fn hash_str_from(mut state: u32, s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        state ^= bytes[i] as u32;
        state = state.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    state
}
