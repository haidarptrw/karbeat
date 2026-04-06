use std::sync::Arc;

use crate::{ audio::engine::PlaybackMode, commands::AudioCommand, context::utils::try_send_audio_command_chain, core::project::{ GeneratorId, Pattern, PatternId }, lock::get_app_read };

pub fn get_pattern(pattern_id: &PatternId) -> anyhow::Result<Arc<Pattern>> {
    let app = get_app_read();
    let pattern_ref = app.pattern_pool
        .get(pattern_id)
        .ok_or_else(|| anyhow::anyhow!("Pattern {:?} not found", pattern_id))?;

    Ok(pattern_ref.clone())
}

/// Fetches patterns, applies a mapper, and collects into ANY collection type `C`.
pub fn get_patterns<C, Item, F>(mapper: F) -> anyhow::Result<C>
    where
        F: Fn(u32, &Pattern) -> Item, // The mapper takes the ID and the Pattern, and returns an Item
        C: FromIterator<Item> // The collection must be buildable from an iterator of Items
{
    let app = get_app_read();

    let patterns = app.pattern_pool
        .iter()
        .map(|(&id, pattern_arc)| {
            // Let the closure handle exactly what the Item shape looks like
            mapper(id.into(), pattern_arc.as_ref())
        })
        .collect::<C>(); // Collect dynamically resolves to type C

    Ok(patterns)
}

pub fn play_pattern_preview(
    pattern_id: PatternId,
    generator_id: GeneratorId
) -> anyhow::Result<()> {
    {
        let app = get_app_read();
        if !app.pattern_pool.contains_key(&pattern_id) {
            return Err(anyhow::anyhow!("Pattern {:?} not found", pattern_id));
        }
    }

    // Try send command
    {
        // Send commands to switch to Pattern mode and start playing
        try_send_audio_command_chain(
            vec![
                AudioCommand::SetPlaybackMode(PlaybackMode::Pattern {
                    pattern_id,
                    generator_id,
                }),
                AudioCommand::SetPlaying(true)
            ]
        )?;
    }

    Ok(())
}

pub fn stop_pattern_preview() -> anyhow::Result<()> {
    // Send commands to stop playing and switch back to Song mode
    try_send_audio_command_chain(
        vec![AudioCommand::SetPlaying(false), AudioCommand::SetPlaybackMode(PlaybackMode::Song)]
    )
}
