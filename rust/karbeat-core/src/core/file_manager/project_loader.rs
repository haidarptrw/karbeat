use std::{
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
    sync::Arc,
};

use anyhow::Context;
use rayon::prelude::*;
use zip::{read::ZipArchive, write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::core::{
    file_manager::audio_loader::load_audio_file,
    project::{ApplicationState, AudioSourceId, ProjectMetadata},
};

const KARBEAT_MAGIC_HEADER: &[u8; 8] = b"KARBEAT1";

pub fn save_daw_project(save_path: &Path, app_state: &ApplicationState) -> anyhow::Result<()> {
    let mut file = File::create(save_path)?;
    file.write_all(KARBEAT_MAGIC_HEADER)?;
    let metadata_toml = toml::to_string(&app_state.metadata)?;

    let mut zip = ZipWriter::new(file);

    let deflated_options =
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored_options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    zip.start_file("metadata.toml", deflated_options)?;
    zip.write_all(metadata_toml.as_bytes())?;

    // Clone the app state so we can modify file paths for embedded audio
    let mut saveable_state = app_state.clone();
    let library = Arc::make_mut(&mut saveable_state.asset_library);

    zip.add_directory("audio/", stored_options)?;

    // Pre-filter and pre-allocate paths to keep the zip write loop as tight as possible
    let mut valid_sources = Vec::new();
    for (id, audio_arc) in app_state.asset_library.source_map.iter() {
        let path = &audio_arc.file_path;
        if !path.as_os_str().is_empty() && path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("sample.bin");
            let internal_name = format!("audio/{}_{}", id.to_u32(), file_name);
            valid_sources.push((*id, path.clone(), internal_name));
        }
    }

    // Write audio files to zip
    for (id, original_path, internal_name) in valid_sources {
        zip.start_file(&internal_name, stored_options)?;

        let source_audio_file = File::open(&original_path)
            .with_context(|| format!("Failed to open audio file: {}", original_path.display()))?;

        // Use a BufReader with a 128KB chunk size for much faster sequential disk reads
        let mut reader = BufReader::with_capacity(128 * 1024, source_audio_file);
        std::io::copy(&mut reader, &mut zip)?;

        // Rewrite the file path in the cloned state to use the zip-internal path
        if let Some(entry) = library.source_map.get_mut(&id) {
            let waveform = Arc::make_mut(entry);
            waveform.file_path = internal_name.into();
        }
    }

    // Serialize the modified state to MessagePack binary
    let project_msgpack = rmp_serde::to_vec(&saveable_state)
        .context("Failed to serialize project state to MessagePack")?;
    zip.start_file("project.msgpack", deflated_options)?;
    zip.write_all(&project_msgpack)?;

    zip.finish()?;
    Ok(())
}

pub fn load_daw_project(path: &Path) -> anyhow::Result<ApplicationState> {
    let mut file =
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;

    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != KARBEAT_MAGIC_HEADER {
        return Err(anyhow::anyhow!("Invalid or corrupted .karbeat file"));
    }

    let mut archive = ZipArchive::new(file)?;
    let mut project_bytes = Vec::new();
    {
        let mut project_entry = archive
            .by_name("project.msgpack")
            .context("project.msgpack missing from .karbeat archive")?;
        project_entry.read_to_end(&mut project_bytes)?;
    }
    let mut app_state: ApplicationState =
        rmp_serde::from_slice(&project_bytes).context("Failed to deserialize project.msgpack")?;

    let library = Arc::make_mut(&mut app_state.asset_library);
    library.source_map.clear();

    // Create a persistent cache directory for this session, avoiding randomized file names
    // We use into_path() to intentionally leak the directory so it persists during playback
    let cache_dir = tempfile::Builder::new()
        .prefix("karbeat_session_")
        .tempdir()
        .context("Failed to create temporary session cache directory")?
        .keep();

    // PHASE 1: Sequentially extract all audio files to disk (I/O Bound)
    // We collect the paths into a vector to process them concurrently later.
    let mut audio_tasks = Vec::with_capacity(archive.len());

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();
        let Some((id, file_name)) = parse_embedded_audio_path(&name) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }

        let audio_folder = cache_dir.join(id.to_string());
        std::fs::create_dir_all(&audio_folder)?;

        let dest_path = audio_folder.join(&file_name);

        let mut dest_file = File::create(&dest_path).with_context(|| {
            format!(
                "Failed to create extracted audio file at {}",
                dest_path.display()
            )
        })?;

        std::io::copy(&mut entry, &mut dest_file).with_context(|| {
            format!("Failed to extract embedded audio from archive entry {name}")
        })?;

        // Queue for parallel decoding
        audio_tasks.push((id, file_name, dest_path));
    }

    // PHASE 2: Decode all audio files in parallel (CPU Bound)
    let decoded_waveforms: anyhow::Result<Vec<_>> = audio_tasks
        .into_par_iter()
        .map(|(id, file_name, dest_path)| {
            let dest_path_str = dest_path.to_str().with_context(|| {
                format!(
                    "Embedded audio path is not valid UTF-8: {}",
                    dest_path.display()
                )
            })?;

            let mut waveform =
                load_audio_file(dest_path_str, Some(&file_name)).with_context(|| {
                    format!("Failed to decode embedded audio for source id {id} ({file_name})")
                })?;

            waveform
                .try_assign_id(AudioSourceId::from(id))
                .with_context(|| format!("Duplicate or invalid audio source id {id}"))?;

            Ok((id, waveform))
        })
        .collect();

    // Insert all decoded waveforms back into the map safely on the main thread
    for (id, waveform) in decoded_waveforms? {
        library
            .source_map
            .insert(AudioSourceId::from(id), Arc::new(waveform));
    }

    let max_source_id = library
        .source_map
        .keys()
        .map(|k| k.to_u32())
        .max()
        .unwrap_or(0);
    library.next_id = library.next_id.max(max_source_id.saturating_add(1));

    app_state.update_max_sample_index();

    Ok(app_state)
}

pub fn peek_project_metadata(path: &Path) -> anyhow::Result<ProjectMetadata> {
    let mut file = File::open(path)?;

    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != KARBEAT_MAGIC_HEADER {
        return Err(anyhow::anyhow!("Invalid or corrupted .karbeat file"));
    }

    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive
        .by_name("metadata.toml")
        .context("metadata.toml missing from .karbeat archive")?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf)?;
    let metadata: ProjectMetadata = toml::from_str(&buf)?;

    Ok(metadata)
}

/// Parses `audio/{id}_{file_name}` as produced by [`save_karbeat_project`].
fn parse_embedded_audio_path(zip_name: &str) -> Option<(u32, String)> {
    let rest = zip_name.strip_prefix("audio/")?;
    if rest.is_empty() || rest.ends_with('/') {
        return None;
    }
    let (id_str, file_name) = rest.split_once('_')?;
    let id = id_str.parse().ok()?;
    Some((id, file_name.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn it_should_be_able_to_save_project() {
        // Setup isolated temp directory
        let dir = tempdir().expect("Failed to create temp directory");
        let file_path = dir.path().join("test_save.karbeat");

        // Create dummy state
        let app_state = ApplicationState::default();

        // Execute Save
        let result = save_daw_project(&file_path, &app_state);
        assert!(result.is_ok(), "Failed to save project: {:?}", result.err());
        assert!(
            file_path.exists(),
            "The .karbeat file was not created on disk"
        );

        // Verify Custom Magic Header exists at the exact beginning of the file
        let mut file = File::open(&file_path).unwrap();
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, KARBEAT_MAGIC_HEADER, "Magic header did not match");
    }

    #[test]
    fn it_should_reject_invalid_project_file() {
        let dir = tempdir().unwrap();

        // === Scenario A: Missing Magic Header ===
        let file_path_no_magic = dir.path().join("invalid_no_magic.karbeat");
        let mut file = File::create(&file_path_no_magic).unwrap();
        file.write_all(b"GARBAGE_DATA_NO_MAGIC_HEADER").unwrap();

        let load_result = load_daw_project(&file_path_no_magic);
        assert!(load_result.is_err());
        assert_eq!(
            load_result.unwrap_err().to_string(),
            "Invalid or corrupted .karbeat file",
            "Did not fail with the expected magic header error"
        );

        // === Scenario B: Valid Header, but corrupted ZIP payload ===
        let file_path_bad_zip = dir.path().join("invalid_bad_zip.karbeat");
        let mut file2 = File::create(&file_path_bad_zip).unwrap();
        file2.write_all(KARBEAT_MAGIC_HEADER).unwrap();
        file2.write_all(b"THIS IS NOT A ZIP FILE").unwrap();

        let load_result_zip = load_daw_project(&file_path_bad_zip);
        assert!(
            load_result_zip.is_err(),
            "Failed to reject a corrupted ZIP payload"
        );
    }

    #[test]
    fn test_flow_from_save_to_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("flow_test.karbeat");

        // Setup initial state
        let original_state = ApplicationState::default();

        // Save
        let save_result = save_daw_project(&file_path, &original_state);
        assert!(save_result.is_ok(), "Failed to save project in flow test");

        // Peek Metadata
        let peek_result = peek_project_metadata(&file_path);
        assert!(
            peek_result.is_ok(),
            "Failed to peek metadata from saved file"
        );

        // Load & Verify
        let load_result = load_daw_project(&file_path);
        assert!(
            load_result.is_ok(),
            "Failed to load the project we just saved: {:?}",
            load_result.err()
        );

        let loaded_state = load_result.unwrap();

        assert_eq!(
            original_state, loaded_state,
            "Loaded state did not match the saved state!"
        );
    }

    #[test]
    fn it_should_be_able_to_load_valid_project() {
        // Because "validity" in this context requires a valid TOML and ZIP layout,
        // the safest way to test an isolated valid load is to generate a fresh one
        // using the save function, ensuring the loader can parse its own formatting.
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("valid_load.karbeat");
        let app_state = ApplicationState::default();

        save_daw_project(&file_path, &app_state).unwrap();

        let load_result = load_daw_project(&file_path);
        assert!(
            load_result.is_ok(),
            "Failed to load a known-valid project file"
        );
    }
}
