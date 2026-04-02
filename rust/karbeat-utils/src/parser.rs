/// A trait for structs that can be constructed from a custom command JSON payload
pub trait FromPluginCommand: Sized {
    fn from_json(payload: &serde_json::Value) -> Result<Self, String>;
}