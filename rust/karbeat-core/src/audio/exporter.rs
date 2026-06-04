use rtrb::RingBuffer;
use thiserror::Error;

use crate::{
    audio::{
        render_state::AudioRenderState,
        writer::{create_writer, AudioExportConfig, AudioWriter},
    },
    commands::AudioCommand,
    context::utils::send_audio_command,
    core::project::ApplicationState,
};

#[derive(Debug, Clone, Error)]
#[error("Audio export failed ({error_source}): {message}")]
pub struct AudioExportError {
    pub error_source: String,
    pub message: String,
}

impl AudioExportError {
    pub fn new(source: &str, message: impl Into<String>) -> Self {
        Self {
            error_source: source.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum TailHandling {
    #[default]
    CutRemainder,
    LeaveRemainder,
    WrapRemainder,
}

/// Export project to a sound file based on provided writer
/// Generic, UI-agnostic.
/// `progress_callback` should return `true` to continue, or `false` to abort rendering.
pub fn export_project<F>(
    app_state: &ApplicationState,
    output_path: &str,
    config: AudioExportConfig,
    tail_handling: TailHandling,
    mut progress_callback: F,
) -> Result<(), AudioExportError>
where
    F: FnMut(f32) -> bool,
{
    log::info!("Starting offline render to: {}", output_path);
    let path = std::path::Path::new(output_path);

    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;
    let block_size = 4096; // Faster offline rendering

    let mut writer = create_writer(path, config).map_err(|e| {
        AudioExportError::new("WriterInit", format!("Failed to create writer: {}", e))
    })?;

    // Create a static snapshot of the Render State
    let render_state = AudioRenderState::from(app_state);

    // Set up Dummy Communication Channels
    let (mut cmd_producer, cmd_consumer) = RingBuffer::<AudioCommand>::new(1024);
    let (pos_producer, mut _pos_consumer) = RingBuffer::new(1024);
    let (feedback_producer, mut _feedback_consumer) = RingBuffer::new(1024);

    // Create a oneshot channel to receive the cloned engine
    let (engine_tx, engine_rx) = std::sync::mpsc::channel();

    // Send audio command to get a copy of Audio Engine from live engine.
    // The engine is cloned from its own internal graph state — no triple-buffer involved.
    send_audio_command(AudioCommand::QueryAudioEngine {
        command_consumer: cmd_consumer,
        position_producer: pos_producer,
        feedback_producer: feedback_producer,
        response_tx: engine_tx,
    });

    let mut offline_engine = *engine_rx.recv().map_err(|_| {
        AudioExportError::new("QueryEngineReceiver", "Failed to received offline engine")
    })?;

    cmd_producer
        .push(AudioCommand::SetPlaybackMode(
            crate::audio::engine::PlaybackMode::Song,
        ))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;
    cmd_producer
        .push(AudioCommand::SetPlayhead(0))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;
    cmd_producer
        .push(AudioCommand::SetPlaying(true))
        .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;

    let song_length_samples = render_state.graph.max_sample_index;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(4);

    let writer_thread = std::thread::spawn(move || -> Result<(), AudioExportError> {
        // This thread wakes up whenever a new block of audio is ready
        while let Ok(buffer) = rx.recv() {
            writer
                .write(&buffer)
                .map_err(|e| AudioExportError::new("Writer", format!("Write error: {}", e)))?;
        }

        writer
            .finalize()
            .map_err(|e| AudioExportError::new("Writer", format!("Finalize error: {}", e)))?;

        Ok(())
    });

    let mut mix_buffer = vec![0.0; block_size * channels as usize];

    let throttle_limit = (sample_rate / (block_size as u32) / 30).max(1);
    let mut loop_counter = 0;

    // ========================================================================
    // PHASE 1: PRE-ROLL (Only for WrapRemainder)
    // Plays the entire track into memory to prime the delay and reverb buffers
    // ========================================================================
    if matches!(tail_handling, TailHandling::WrapRemainder) {
        log::info!("Pre-rolling engine for WrapRemainder...");
        let mut preroll_processed = 0;

        while preroll_processed < song_length_samples {
            let remaining = song_length_samples - preroll_processed;
            let frames_to_process = std::cmp::min(block_size as u32, remaining) as usize;
            let samples_to_process = frames_to_process * (channels as usize);

            let active_slice = &mut mix_buffer[..samples_to_process];
            offline_engine.process(active_slice);

            // Discard the audio, just clear queues
            while let Ok(_) = _pos_consumer.pop() {}
            while let Ok(_) = _feedback_consumer.pop() {}

            preroll_processed += frames_to_process as u32;
            loop_counter += 1;

            if loop_counter % throttle_limit == 0 {
                // UI Progress (0% to 50%)
                let progress = ((preroll_processed as f32) / (song_length_samples as f32)) * 0.5;
                if !progress_callback(progress) {
                    return Ok(());
                }
            }
        }

        // Reset playhead for the actual render. We do NOT recreate plugins,
        // ensuring their delay buffers carry over seamlessly into the actual render.
        cmd_producer
            .push(AudioCommand::SetPlayhead(0))
            .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;
    }

    // ========================================================================
    // PHASE 2: MAIN RENDER
    // ========================================================================
    log::info!("Rendering main track bounds...");
    let mut processed_samples: u32 = 0;

    // The "Faster-Than-Realtime" Loop
    while processed_samples < song_length_samples {
        let remaining = song_length_samples - processed_samples;
        let frames_to_process = std::cmp::min(block_size as u32, remaining) as usize;
        let samples_to_process = frames_to_process * (channels as usize);

        // Process the exact slice needed
        let active_slice = &mut mix_buffer[..samples_to_process];
        offline_engine.process(active_slice);

        if tx.send(active_slice.to_vec()).is_err() {
            break; // Stop rendering if the writer thread panicked/crashed
        }

        // Keep the position/feedback queues from filling up and blocking
        while let Ok(_) = _pos_consumer.pop() {}
        while let Ok(_) = _feedback_consumer.pop() {}

        processed_samples += frames_to_process as u32;
        loop_counter += 1;

        // Callback reporting
        if loop_counter % throttle_limit == 0 {
            // Adjust progress scaling based on the mode
            let base_progress = if matches!(tail_handling, TailHandling::WrapRemainder) {
                0.5
            } else {
                0.0
            };
            let progress_scale = if matches!(tail_handling, TailHandling::LeaveRemainder) {
                0.95
            } else {
                0.5
            };

            let progress = base_progress
                + ((processed_samples as f32) / (song_length_samples as f32)) * progress_scale;
            if !progress_callback(progress) {
                log::warn!("Export cancelled by callback.");
                return Ok(());
            }
        }
    }

    // ========================================================================
    // PHASE 3: DYNAMIC TAIL RENDERING (Only for LeaveRemainder)
    // Detects when the reverb/delay naturally dies out
    // ========================================================================
    if matches!(tail_handling, TailHandling::LeaveRemainder) {
        log::info!("Calculating exact plugin tail (LeaveRemainder)...");

        // We must stop the transport first! This triggers the engine's internal
        // "stop_all_active_generators" logic which queues NoteOffs,
        // effectively starting the final ADSR release phase.
        cmd_producer
            .push(AudioCommand::SetPlaying(false))
            .map_err(|_| AudioExportError::new("Engine", "Command queue full"))?;

        // Process one empty block just to let the engine digest the SetPlaying(false) command
        // and initialize the track_tails in its internal state.
        offline_engine.process(&mut mix_buffer[..block_size * (channels as usize)]);
        if tx
            .send(mix_buffer[..block_size * (channels as usize)].to_vec())
            .is_err()
        {
            return Ok(());
        }

        // Query all loaded plugins for their tail size
        let mut max_tail_samples: u32 = 0;

        let plugin_state = offline_engine.plugin_state();

        for effect in &plugin_state.master_effects {
            max_tail_samples = max_tail_samples.max(effect.plugin.tail_samples());
        }

        for bus_chain in plugin_state.bus_effects.iter() {
            for effect in bus_chain.iter() {
                max_tail_samples = max_tail_samples.max(effect.plugin.tail_samples());
            }
        }

        for track_chain in plugin_state.track_effects.iter() {
            for effect in track_chain.iter() {
                max_tail_samples = max_tail_samples.max(effect.plugin.tail_samples());
            }
        }

        for gen in plugin_state.generators.iter() {
            if let Some(ref gen_instance) = gen {
                let tail = gen_instance.plugin.tail_samples();
                // Synths often return u32::MAX for infinite sustain until NoteOff.
                // Since we just sent a NoteOff by stopping transport, we clamp this
                // to a reasonable default if it hasn't properly
                // calculated a finite release tail yet.
                if tail == u32::MAX {
                    max_tail_samples = max_tail_samples.max(sample_rate * 20);
                } else {
                    max_tail_samples = max_tail_samples.max(tail);
                }
            }
        }

        // Hard fallback cap at 60 seconds just in case a plugin reports something insane
        max_tail_samples = max_tail_samples.min(sample_rate * 60);

        log::info!("Maximum tail calculated as {} samples", max_tail_samples);

        // Now simply render exactly that many samples!
        let mut tail_processed = 0;

        while tail_processed < max_tail_samples {
            let remaining = max_tail_samples - tail_processed;
            let frames_to_process = std::cmp::min(block_size as u32, remaining) as usize;
            let samples_to_process = frames_to_process * (channels as usize);

            let active_slice = &mut mix_buffer[..samples_to_process];

            // Because transport is stopped, the engine will just pull from
            // the ringing effects and fading synths without advancing the sequencer.
            offline_engine.process(active_slice);

            if tx.send(active_slice.to_vec()).is_err() {
                break;
            }

            while let Ok(_) = _pos_consumer.pop() {}
            while let Ok(_) = _feedback_consumer.pop() {}

            tail_processed += frames_to_process as u32;
        }
        log::info!("Tail rendering complete.");
    }

    progress_callback(1.0);

    drop(tx);

    writer_thread
        .join()
        .map_err(|_| AudioExportError::new("Thread", "Writer thread panicked"))??;

    log::info!("Offline render successfully completed!");
    Ok(())
}
