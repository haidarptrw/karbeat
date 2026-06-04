use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DawErrorCode {
    TrackNotFound,
    InvalidTrackName,
    InvalidColorFormat,
    IoError,
    InternalError,
}

/// The uniform error struct sent to Flutter
#[derive(Debug, Error)]
#[error("[{code:?}] {reason}")]
pub struct DawError {
    pub code: DawErrorCode,
    pub reason: String,

    // Captures the underlying stack trace or library error for debugging
    pub cause: Option<String>,
}

impl From<anyhow::Error> for DawError {
    fn from(err: anyhow::Error) -> Self {
        DawError {
            code: DawErrorCode::InternalError,
            reason: "An internal engine error occurred".to_string(),
            cause: Some(err.to_string()),
        }
    }
}
