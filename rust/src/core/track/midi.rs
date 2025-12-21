use crate::core::project::{Note, Pattern};

impl Pattern {
    /// Sort notes by start time (in-place)
    pub fn sort_notes(&mut self) {
        self.notes.sort();
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
    }

    /// Get notes sorted by start time without modifying the pattern
    pub fn notes_by_time(&self) -> Vec<&Note> {
        let mut refs: Vec<&Note> = self.notes.iter().collect();
        refs.sort_by_key(|n| n.start_tick);
        refs
    }

    pub fn add_note(
        &mut self,
        key: u8,
        start_tick: u64,
        duration: Option<u64>,
    ) -> anyhow::Result<Note> {
        // Validate MIDI key range (0-127)
        if key > 127 {
            return Err(anyhow::anyhow!(
                "MIDI key must be between 0 and 127, got {}",
                key
            ));
        }

        let duration_proper = duration.unwrap_or(960); // standard default of 960 ticks per beat

        // TODO: Handle pattern expansion (Moyou Tenkai 🤞 (模様展開)
        // upon insertion of a note outside length ticks boundary)
        if start_tick >= self.length_ticks {
            return Err(anyhow::anyhow!(
                "start_tick ({}) must be less than pattern length ({})",
                start_tick,
                self.length_ticks
            ));
        }

        if duration_proper == 0 {
            return Err(anyhow::anyhow!("Note duration must be greater than 0"));
        }

        let new_note_id = self.next_note_id;
        self.next_note_id += 1;

        let note = Note {
            id: new_note_id,
            start_tick,
            duration: duration_proper,
            key,
            velocity: 100,
            probability: 1.0,
            micro_offset: 0,
            mute: false,
        };

        self.notes.push(note.clone());

        self.sort_notes_unstable();
        Ok(note)
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

        Ok(self.notes.remove(index))
    }

    /// Delete note by matching start_tick and key
    /// Returns the number of notes deleted
    pub fn delete_note_by_params(&mut self, start_tick: u64, key: u8) -> usize {
        let initial_len = self.notes.len();
        self.notes
            .retain(|n| !(n.start_tick == start_tick && n.key == key));
        initial_len - self.notes.len()
    }

    /// Delete all notes within a time range
    pub fn delete_notes_in_range(&mut self, start_tick: u64, end_tick: u64) -> usize {
        let initial_len = self.notes.len();
        self.notes
            .retain(|n| n.start_tick < start_tick || n.start_tick >= end_tick);
        initial_len - self.notes.len()
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
        Ok(&self.notes[index])
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

        // Validate Timing
        if new_start_tick >= self.length_ticks {
            return Err(anyhow::anyhow!(
                "start_tick ({}) must be less than pattern length ({})",
                new_start_tick,
                self.length_ticks
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
    }

    /// Clone a note and add it at a different time
    pub fn duplicate_note(&mut self, index: usize, new_start_tick: u64) -> anyhow::Result<Note> {
        if index >= self.notes.len() {
            return Err(anyhow::anyhow!(
                "Note index {} out of bounds (pattern has {} notes)",
                index,
                self.notes.len()
            ));
        }

        if new_start_tick >= self.length_ticks {
            return Err(anyhow::anyhow!(
                "start_tick ({}) must be less than pattern length ({})",
                new_start_tick,
                self.length_ticks
            ));
        }

        let mut new_note = self.notes[index].clone();
        new_note.start_tick = new_start_tick;

        self.notes.push(new_note.clone());
        self.sort_notes_unstable();

        Ok(new_note)
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
            let new_key = note.key as i16 + semitones;

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
