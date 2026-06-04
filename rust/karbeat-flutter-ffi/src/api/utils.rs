use flutter_rust_bridge::frb;

/// Helper function to acquire u32 ID from string
/// using FNV1a hash functions
#[frb(sync)]
pub const fn hash_str_fnv1a(s: &str) -> u32 {
    karbeat_utils::hash::hash_str(s)
}
