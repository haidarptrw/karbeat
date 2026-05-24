use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::project::ApplicationState;
use crate::core::project::ClipboardContent;
use crate::core::project::Note;
use crate::core::project::NoteId;
use crate::shared::id::PatternId;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    pub length_ticks: u64,

    pub notes: Vec<Note>,

    pub next_note_id: u32,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            id: PatternId::default(),
            name: "Pattern".to_string(),
            length_ticks: 0,
            notes: Vec::new(),
            next_note_id: 0,
        }
    }
}

impl Pattern {
    /// Recalculate the pattern length to be exactly the end of the last note.
    pub fn recalculate_length(&mut self) {
        let max_end = self
            .notes
            .iter()
            .map(|n| n.start_tick + n.duration)
            .max()
            .unwrap_or(0);
        self.length_ticks = max_end;
    }

    /// Sort notes by start time (in-place)
    pub fn sort_notes(&mut self) {
        self.notes.sort();
        self.recalculate_length();
    }

    /// Sort notes by start time and return a new sorted vector
    pub fn sorted_notes(&self) -> Vec<Note> {
        let mut sorted = self.notes.clone();
        sorted.sort();
        sorted
    }

    /// Sort notes by start time (unstable, potentially faster)
    pub fn sort_notes_unstable(&mut self) {
        self.notes.sort_unstable();
        self.recalculate_length();
    }

    /// Get notes sorted by start time without modifying the pattern
    pub fn notes_by_time(&self) -> Vec<&Note> {
        let mut refs: Vec<&Note> = self.notes.iter().collect();
        refs.sort_by_key(|n| n.start_tick);
        refs
    }

    /// - Generates a new unique ID (ensuring validity).
    /// - Auto-expands pattern length if the note goes out of bounds.
    /// - Sorts the notes.
    pub fn insert_note(&mut self, mut note: Note) -> anyhow::Result<Note> {
        // 1. Validation
        if note.key > 127 {
            return Err(anyhow::anyhow!("MIDI key must be 0-127, got {}", note.key));
        }
        if note.duration == 0 {
            return Err(anyhow::anyhow!("Note duration must be > 0"));
        }

        // 3. ID Generation (Critical for Validity)
        note.id = NoteId::next(&mut self.next_note_id);

        // 4. Push & Sort
        self.notes.push(note.clone());
        self.sort_notes_unstable();
        // recalculate_length is called in sort_notes_unstable

        Ok(note)
    }

    pub fn add_note(
        &mut self,
        key: u8,
        start_tick: u64,
        duration: Option<u64>,
    ) -> anyhow::Result<Note> {
        let duration_proper = duration.unwrap_or(960);
        // Construct the note (ID will be overridden by insert_note)
        let note = Note {
            id: NoteId::default(), // Placeholder, will be assigned by insert_note
            start_tick,
            duration: duration_proper,
            key,
            velocity: 100,
            probability: 1.0,
            micro_offset: 0,
            mute: false,
        };

        // Use central method
        self.insert_note(note)
    }

    /// Delete a note at the specified index
    /// Returns the deleted note or an error if index is out of bounds
    pub fn delete_note(&mut self, index: usize) -> anyhow::Result<Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        let removed = self.notes.remove(index);
        self.recalculate_length();
        Ok(removed)
    }

    /// Delete note by matching start_tick and key
    /// Returns the number of notes deleted
    pub fn delete_note_by_params(&mut self, start_tick: u64, key: u8) -> usize {
        let initial_len = self.notes.len();
        self.notes
            .retain(|n| !(n.start_tick == start_tick && n.key == key));
        if self.notes.len() < initial_len {
            self.recalculate_length();
        }
        initial_len - self.notes.len()
    }

    /// Delete all notes within a time range
    pub fn delete_notes_in_range(&mut self, start_tick: u64, end_tick: u64) -> usize {
        let initial_len = self.notes.len();
        self.notes
            .retain(|n| n.start_tick < start_tick || n.start_tick >= end_tick);
        if self.notes.len() < initial_len {
            self.recalculate_length();
        }
        initial_len - self.notes.len()
    }

    pub fn delete_notes_by_id(&mut self, note_ids: Arc<[NoteId]>) -> usize {
        let initial_len = self.notes.len();
        self.notes.retain(|n| !note_ids.contains(&n.id));
        if self.notes.len() < initial_len {
            self.recalculate_length();
        }
        initial_len - self.notes.len()
    }

    /// Efficiently adds multiple notes and sorts only once at the end.
    pub fn insert_notes_batch(&mut self, new_notes: Vec<Note>) -> anyhow::Result<Vec<Note>> {
        let mut inserted = Vec::with_capacity(new_notes.len());
        for mut note in new_notes {
            if note.key > 127 || note.duration == 0 {
                continue; // Safely skip invalid notes during a batch drag
            }
            note.id = NoteId::next(&mut self.next_note_id);
            self.notes.push(note.clone());
            inserted.push(note);
        }

        // Sort and recalculate timeline boundaries exactly once
        self.sort_notes_unstable();
        Ok(inserted)
    }

    /// Efficiently moves multiple notes and sorts only once at the end.
    pub fn move_notes_batch(
        &mut self,
        updates: &[(NoteId, u64, u8)],
    ) -> anyhow::Result<Vec<(Note, u64, u8)>> {
        let mut results = Vec::new();

        for &(note_id, new_start_tick, new_key) in updates {
            if new_key > 127 {
                continue;
            }
            if let Some(index) = self.notes.iter().position(|n| n.id == note_id) {
                let old_tick = self.notes[index].start_tick;
                let old_key = self.notes[index].key;

                self.notes[index].start_tick = new_start_tick;
                self.notes[index].key = new_key;

                results.push((self.notes[index].clone(), old_tick, old_key));
            }
        }

        if !results.is_empty() {
            self.sort_notes_unstable();
        }

        Ok(results)
    }

    /// Resize a note's duration
    /// Returns the modified note or an error if index is invalid
    pub fn resize_note(&mut self, index: usize, new_duration: u64) -> anyhow::Result<&Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        if new_duration == 0 {
            return Err(anyhow::anyhow!("Note duration must be greater than 0"));
        }

        self.notes[index].duration = new_duration;
        self.recalculate_length();
        Ok(&self.notes[index])
    }

    /// Efficiently resizes multiple notes and recalculates length only once.
    pub fn resize_notes_batch(
        &mut self,
        updates: &[(NoteId, u64)],
    ) -> anyhow::Result<Vec<(Note, u64)>> {
        let mut results = Vec::new();

        for &(note_id, new_duration) in updates {
            if new_duration == 0 {
                continue;
            }
            if let Some(index) = self.notes.iter().position(|n| n.id == note_id) {
                let old_duration = self.notes[index].duration;
                self.notes[index].duration = new_duration;

                results.push((self.notes[index].clone(), old_duration));
            }
        }

        if !results.is_empty() {
            self.recalculate_length();
        }

        Ok(results)
    }

    pub fn move_note(
        &mut self,
        index: usize,
        new_start_tick: u64,
        new_key: u8,
    ) -> anyhow::Result<&Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        // Validate Key
        if new_key > 127 {
            return Err(anyhow::anyhow!(
                "MIDI key must be between 0 and 127, got {}",
                new_key
            ));
        }

        // Validate Key
        if new_key > 127 {
            return Err(anyhow::anyhow!(
                "MIDI key must be between 0 and 127, got {}",
                new_key
            ));
        }

        // Update the note
        self.notes[index].start_tick = new_start_tick;
        self.notes[index].key = new_key;

        // Re-sort to maintain chronological order in the vector
        self.sort_notes_unstable();

        // Retrieve reference to the updated note
        // We search by both tick and key to ensure we find the correct note (or an identical one)
        let note = self
            .notes
            .iter()
            .find(|n| n.start_tick == new_start_tick && n.key == new_key)
            .ok_or_else(|| anyhow::anyhow!("Note not found after moving"))?;

        Ok(note)
    }

    pub fn set_note_params(
        &mut self,
        index: usize,
        velocity: Option<u8>,
        probability: Option<f32>,
        micro_offset: Option<i8>,
        mute: Option<bool>,
    ) -> anyhow::Result<&Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        let note = &mut self.notes[index];

        // Update each parameter if provided
        if let Some(v) = velocity {
            if v > 127 {
                return Err(anyhow::anyhow!(
                    "Velocity must be between 0 and 127, got {}",
                    v
                ));
            }
            note.velocity = v;
        }

        if let Some(p) = probability {
            if !(0.0..=1.0).contains(&p) {
                return Err(anyhow::anyhow!(
                    "Probability must be between 0.0 and 1.0, got {}",
                    p
                ));
            }
            note.probability = p;
        }

        if let Some(o) = micro_offset {
            note.micro_offset = o;
        }

        if let Some(m) = mute {
            note.mute = m;
        }

        Ok(&self.notes[index])
    }

    /// Update a note's key (pitch)
    pub fn set_note_key(&mut self, index: usize, key: u8) -> anyhow::Result<&Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        if key > 127 {
            return Err(anyhow::anyhow!(
                "MIDI key must be between 0 and 127, got {}",
                key
            ));
        }

        self.notes[index].key = key;
        Ok(&self.notes[index])
    }

    /// Get notes that overlap with a specific time range
    pub fn get_notes_in_range(&self, start_tick: u64, end_tick: u64) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|n| {
                let note_end = n.start_tick + n.duration;
                // Note overlaps if it starts before range ends AND ends after range starts
                n.start_tick < end_tick && note_end > start_tick
            })
            .collect()
    }

    /// Get the note at a specific index
    pub fn get_note(&self, index: usize) -> Option<&Note> {
        self.notes.get(index)
    }

    /// Get mutable reference to a note at a specific index
    pub fn get_note_mut(&mut self, index: usize) -> Option<&mut Note> {
        self.notes.get_mut(index)
    }

    /// Find notes by key (pitch)
    pub fn find_notes_by_key(&self, key: u8) -> Vec<&Note> {
        self.notes.iter().filter(|n| n.key == key).collect()
    }

    /// Count total notes in pattern
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Clear all notes from pattern
    pub fn clear_notes(&mut self) {
        self.notes.clear();
        self.recalculate_length();
    }

    /// Used by Undo/Redo to insert a specific note state.
    /// Preserves the Note ID and ensures Pattern consistency (Sort/Expand).
    pub fn restore_note(&mut self, note: Note) -> anyhow::Result<()> {
        // 1. Validate
        if note.key > 127 {
            return Err(anyhow::anyhow!("Invalid key {}", note.key));
        }

        // 2. Insert directly (Preserving ID)
        self.notes.push(note);

        // 3. Maintain Order
        self.sort_notes_unstable();

        Ok(())
    }

    /// Clone a note and add it at a different time
    pub fn duplicate_note(&mut self, index: usize, new_start_tick: u64) -> anyhow::Result<Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!("Index out of bounds"));
        }

        // Clone the note to duplicate
        let mut new_note = self.notes[index].clone();
        new_note.start_tick = new_start_tick;

        // Use insert_note to ensure it gets a FRESH ID
        self.insert_note(new_note)
    }

    /// Quantize note start times to a grid
    /// grid_size: snap to multiples of this tick value (e.g., 96 for 16th notes at 960 PPQ)
    pub fn quantize_notes(&mut self, grid_size: u64) {
        if grid_size == 0 {
            return;
        }

        for note in &mut self.notes {
            note.start_tick = (note.start_tick / grid_size) * grid_size;
        }

        self.sort_notes_unstable();
    }

    /// Transpose all notes by a number of semitones
    pub fn transpose(&mut self, semitones: i16) -> anyhow::Result<()> {
        for note in &mut self.notes {
            let new_key = (note.key as i16) + semitones;

            if new_key < 0 || new_key > 127 {
                return Err(anyhow::anyhow!(
                    "Transposition would move note {} outside valid MIDI range (0-127)",
                    note.key
                ));
            }

            note.key = new_key as u8;
        }

        Ok(())
    }
}

impl ApplicationState {
    pub fn add_note_to_pattern(
        &mut self,
        pattern_id: PatternId,
        key: u8,
        start_tick: u64,
        duration: Option<u64>,
    ) -> anyhow::Result<Note> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern: &mut Pattern = Arc::make_mut(pattern_arc);

        let note = pattern.add_note(key, start_tick, duration)?;

        Ok(note)
    }

    pub fn delete_note_from_pattern(
        &mut self,
        pattern_id: PatternId,
        note_id: NoteId,
    ) -> anyhow::Result<Note> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let index = pattern
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or_else(|| anyhow::anyhow!("Note with ID {:?} not found", note_id))?;

        let note = pattern.delete_note(index)?;

        Ok(note)
    }

    pub fn resize_note_in_pattern(
        &mut self,
        pattern_id: PatternId,
        note_id: NoteId,
        new_duration: u64,
    ) -> anyhow::Result<(Note, u64)> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let index = pattern
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or_else(|| anyhow::anyhow!("Note with ID {:?} not found", note_id))?;

        let old_duration = pattern.notes[index].duration;
        let note = pattern.resize_note(index, new_duration)?.clone();

        Ok((note, old_duration))
    }

    pub fn move_note_in_pattern(
        &mut self,
        pattern_id: PatternId,
        note_id: NoteId,
        new_start_tick: u64,
        new_key: u8,
    ) -> anyhow::Result<(Note, u64, u8)> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let index = pattern
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or_else(|| anyhow::anyhow!("Note with ID {:?} not found", note_id))?;

        let old_tick = pattern.notes[index].start_tick;
        let old_key = pattern.notes[index].key;

        let note = pattern.move_note(index, new_start_tick, new_key)?.clone();

        Ok((note, old_tick, old_key))
    }

    pub fn change_note_params_in_pattern(
        &mut self,
        pattern_id: PatternId,
        note_id: NoteId,
        velocity: Option<u8>,
        probability: Option<f32>,
        micro_offset: Option<i8>,
        mute: Option<bool>,
    ) -> anyhow::Result<Note> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let index = pattern
            .notes
            .iter()
            .position(|n| n.id == note_id)
            .ok_or_else(|| anyhow::anyhow!("Note with ID {:?} not found", note_id))?;

        let note = pattern
            .set_note_params(index, velocity, probability, micro_offset, mute)?
            .clone();
        Ok(note)
    }

    pub fn add_notes_to_pattern_batch(
        &mut self,
        pattern_id: PatternId,
        notes_data: &[(u8, u64, Option<u64>)],
    ) -> anyhow::Result<Vec<Note>> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let mut notes_to_insert = Vec::with_capacity(notes_data.len());
        for &(key, start_tick, duration) in notes_data {
            notes_to_insert.push(Note {
                id: NoteId::default(), // Automatically overwritten by pattern.insert_notes_batch
                start_tick,
                duration: duration.unwrap_or(960),
                key,
                velocity: 100,
                probability: 1.0,
                micro_offset: 0,
                mute: false,
            });
        }

        pattern.insert_notes_batch(notes_to_insert)
    }

    pub fn delete_notes_from_pattern_batch(
        &mut self,
        pattern_id: PatternId,
        note_ids: &[NoteId],
    ) -> anyhow::Result<Vec<Note>> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        let deleted_notes: Vec<Note> = pattern
            .notes
            .iter()
            .filter(|n| note_ids.contains(&n.id))
            .cloned()
            .collect();

        // Relies on your existing `delete_notes_by_id` implementation
        pattern.delete_notes_by_id(note_ids.into());

        Ok(deleted_notes)
    }

    pub fn move_notes_in_pattern_batch(
        &mut self,
        pattern_id: PatternId,
        updates: &[(NoteId, u64, u8)],
    ) -> anyhow::Result<Vec<(Note, u64, u8)>> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        pattern.move_notes_batch(updates)
    }

    pub fn resize_notes_in_pattern_batch(
        &mut self,
        pattern_id: PatternId,
        updates: &[(NoteId, u64)],
    ) -> anyhow::Result<Vec<(Note, u64)>> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", pattern_id.to_u32()))?;
        let pattern = Arc::make_mut(pattern_arc);

        pattern.resize_notes_batch(updates)
    }

    pub fn copy_pattern_notes_batch(
        &mut self,
        pattern_id: PatternId,
        note_ids: &[NoteId],
    ) -> anyhow::Result<()> {
        let pattern = self
            .pattern_pool
            .get(&pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {:?} not found", pattern_id))?;

        // Filter and clone the requested notes
        let notes_to_copy: Vec<Note> = pattern
            .notes
            .iter()
            .filter(|n| note_ids.contains(&n.id))
            .cloned()
            .collect();

        // Update the App's clipboard state
        if !notes_to_copy.is_empty() {
            self.clipboard = ClipboardContent::Notes(notes_to_copy);
        } else {
            self.clipboard = ClipboardContent::Empty;
        }

        Ok(())
    }

    pub fn paste_pattern_notes_batch(
        &mut self,
        target_pattern_id: &PatternId,
        left_bound_tick: u64,
        left_note_key: Option<u8>,
    ) -> anyhow::Result<Vec<Note>> {
        let pattern_arc = self
            .pattern_pool
            .get_mut(target_pattern_id)
            .ok_or_else(|| anyhow::anyhow!("Pattern {} not found", target_pattern_id.to_u32()))?;

        // Get clipboard
        let clipboard = &self.clipboard;

        // paste into the pattern
        let ClipboardContent::Notes(notes_vec) = clipboard else {
            return Ok(Vec::new());
        };

        if notes_vec.is_empty() {
            return Ok(Vec::new());
        }

        #[allow(clippy::unwrap_used)]
        let earliest_note = notes_vec.iter().min_by_key(|n| n.start_tick).unwrap();

        let min_tick = earliest_note.start_tick;
        let tick_offset = (left_bound_tick as i64) - (min_tick as i64);

        // If a target key is provided, calculate the pitch shift relative to the earliest note
        let key_offset = if let Some(target_key) = left_note_key {
            (target_key as i16) - (earliest_note.key as i16)
        } else {
            0
        };

        let mut notes_to_insert = Vec::with_capacity(notes_vec.len());
        for note in notes_vec {
            let mut new_note = note.clone();

            // Apply bounds and clamps
            new_note.start_tick = ((new_note.start_tick as i64) + tick_offset).max(0) as u64;
            new_note.key = ((new_note.key as i16) + key_offset).clamp(0, 127) as u8;

            notes_to_insert.push(new_note);
        }

        let pattern_mut = Arc::make_mut(pattern_arc);
        let inserted_notes = pattern_mut.insert_notes_batch(notes_to_insert)?;

        Ok(inserted_notes)
    }

    pub fn cut_pattern_notes_batch(
        &mut self,
        pattern_id: PatternId,
        note_ids: &[NoteId],
    ) -> anyhow::Result<Vec<Note>> {
        // Copy the targeted notes to the internal clipboard
        self.copy_pattern_notes_batch(pattern_id, note_ids)?;

        // Delete them from the pattern and return the deleted notes
        // (so the API wrapper can add them to the undo/redo history)
        self.delete_notes_from_pattern_batch(pattern_id, note_ids)
    }
}
