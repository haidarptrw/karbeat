use std::default;

use crate::core::project::{Clip, Note};

// Enum to hold different types of copied data
#[derive(Clone, Debug, Default)]
pub enum ClipboardContent {
    #[default]
    Empty,
    Notes(Vec<Note>), // A list of notes (for Pattern View)
    Clips(Vec<Clip>), // A list of clips (for Track View)
}