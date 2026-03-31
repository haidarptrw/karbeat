use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

use anyhow::Context;
use zip::{read::ZipArchive, write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::core::{
    file_manager::audio_loader::load_audio_file,
    project::{ApplicationState, AudioSourceId, ProjectMetadata},
};

const KARBEAT_MAGIC_HEADER: &[u8; 8] = b"KARBEAT1";

// FIXME: Fix it so that when closing the app and then reopen the saved project, would not caused the project to be empty
pub fn save_karbeat_project(save_path: &Path, app_state: &ApplicationState) -> anyhow::Result<()> {
    let mut file = File::create(save_path)?;
    file.write_all(KARBEAT_MAGIC_HEADER)?;
    let metadata_toml = toml::to_string(&app_state.metadata)?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("metadata.toml", options)?;
    zip.write_all(metadata_toml.as_bytes())?;

    // Clone the app state so we can modify file paths for embedded audio
    let mut saveable_state = app_state.clone();
    let library = Arc::make_mut(&mut saveable_state.asset_library);

    zip.add_directory("audio/", options)?;

    for (id, audio_arc) in app_state.asset_library.source_map.iter() {
        let wave = audio_arc.as_ref();
        let path = &wave.file_path;
        if path.as_os_str().is_empty() || !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample.bin");

        let internal_name = format!("audio/{}_{}", id.to_u32(), file_name);
        zip.start_file(&internal_name, options)?;

        let mut source_audio_file = File::open(&audio_arc.file_path).with_context(|| {
            format!(
                "Failed to open audio file: {}",
                audio_arc.file_path.display()
            )
        })?;
        std::io::copy(&mut source_audio_file, &mut zip)?;

        // Rewrite the file path in the cloned state to use the zip-internal path
        if let Some(entry) = library.source_map.get_mut(id) {
            let waveform = Arc::make_mut(entry);
            waveform.file_path = internal_name.into();
        }
    }

    // Serialize the modified state to MessagePack binary
    let project_msgpack = rmp_serde::to_vec(&saveable_state)
        .context("Failed to serialize project state to MessagePack")?;
    zip.start_file("project.msgpack", options)?;
    zip.write_all(&project_msgpack)?;

    zip.finish()?;
    Ok(())
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

// FIXME: This is currently failing to load at all. it does not throw an error, but the
pub fn load_karbeat_project(path: &Path) -> anyhow::Result<ApplicationState> {
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

        let suffix = Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        let mut tmp = tempfile::Builder::new()
            .suffix(&suffix)
            .tempfile()
            .context("Failed to create temp file for embedded audio")?;
        std::io::copy(&mut entry, &mut tmp).with_context(|| {
            format!("Failed to extract embedded audio from archive entry {name}")
        })?;
        tmp.as_file_mut().sync_all()?;

        let tmp_path = tmp.path().to_str().with_context(|| {
            format!(
                "Embedded audio path is not valid UTF-8: {}",
                tmp.path().display()
            )
        })?;

        let mut waveform = load_audio_file(tmp_path, Some(&file_name)).with_context(|| {
            format!("Failed to decode embedded audio for source id {id} ({file_name})")
        })?;
        waveform
            .try_assign_id(AudioSourceId::from(id))
            .with_context(|| format!("Duplicate or invalid audio source id {id}"))?;

        // Detach the temp file to prevent it from being deleted after extraction.
        // This ensures the audio files remain on disk so subsequent saves can read them.
        tmp.keep()
            .map_err(|e| anyhow::anyhow!("Failed to persist temp file: {}", e.error))?;

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
        let result = save_karbeat_project(&file_path, &app_state);
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

        let load_result = load_karbeat_project(&file_path_no_magic);
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

        let load_result_zip = load_karbeat_project(&file_path_bad_zip);
        assert!(
            load_result_zip.is_err(),
            "Failed to reject a corrupted ZIP payload"
        );
    }

    #[test]
    fn test_flow_from_save_to_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("flow_test.karbeat");

        // 1. Setup initial state
        let original_state = ApplicationState::default();

        // 2. Save
        let save_result = save_karbeat_project(&file_path, &original_state);
        assert!(save_result.is_ok(), "Failed to save project in flow test");

        // 3. Peek Metadata
        let peek_result = peek_project_metadata(&file_path);
        assert!(
            peek_result.is_ok(),
            "Failed to peek metadata from saved file"
        );

        // 4. Load & Verify
        let load_result = load_karbeat_project(&file_path);
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

        save_karbeat_project(&file_path, &app_state).unwrap();

        let load_result = load_karbeat_project(&file_path);
        assert!(
            load_result.is_ok(),
            "Failed to load a known-valid project file"
        );
    }
}
