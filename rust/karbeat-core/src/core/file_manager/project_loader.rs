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

pub fn save_karbeat_project(
    save_path: &Path,
    app_state: &ApplicationState,
) -> anyhow::Result<()> {
    let mut file = File::create(save_path)?;
    file.write_all(KARBEAT_MAGIC_HEADER)?;
    let metadata_toml = toml::to_string(&app_state.metadata)?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("metadata.toml", options)?;
    zip.write_all(metadata_toml.as_bytes())?;

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

        let mut source_audio_file = File::open(&audio_arc.file_path)
        .with_context(|| format!("Failed to open audio file: {}", audio_arc.file_path.display()))?;
        std::io::copy(&mut source_audio_file, &mut zip)?;
    }

    let project_toml = toml::to_string(&app_state)?;
    zip.start_file("project.toml", options)?;
    zip.write_all(project_toml.as_bytes())?;

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

pub fn load_karbeat_project(path: &Path) -> anyhow::Result<ApplicationState> {
    let mut file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;

    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != KARBEAT_MAGIC_HEADER {
        return Err(anyhow::anyhow!("Invalid or corrupted .karbeat file"));
    }

    let mut archive = ZipArchive::new(file)?;
    let mut project_toml = String::new();
    {
        let mut project_entry = archive
            .by_name("project.toml")
            .context("project.toml missing from .karbeat archive")?;
        project_entry.read_to_string(&mut project_toml)?;
    }
    let mut app_state: ApplicationState = toml::from_str(&project_toml)
        .context("Failed to deserialize project.toml")?;

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