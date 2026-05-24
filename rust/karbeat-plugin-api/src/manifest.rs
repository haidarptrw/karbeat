use std::{fs, path::PathBuf};

use heck::ToSnakeCase;
use karbeat_plugin_types::ParameterSpec;
use serde::Serialize;

/// Represents the exact JSON structure Flutter expects
#[derive(Serialize)]
pub struct PluginManifest {
    pub id: u32,
    pub name: String,
    pub internal_type: String,
    pub is_synth: bool,
    pub parameters: Vec<ParameterSpec>,
}

/// Interface for plugins that are exportable as JSON Manifest
pub trait Manifestable {
    /// Generate the structural manifest for this plugin
    fn build_manifest() -> PluginManifest;

    /// Automatically handles routing the JSON to the correct folder and saving it
    fn export_manifest(base_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut manifest = Self::build_manifest();

        // SANITIZATION PASS
        for param in &mut manifest.parameters {
            // Helper closure to round to 4 decimal places to kill IEEE 754 garbage
            let clean_float = |val: f64| -> f64 { (val * 10000.0).round() / 10000.0 };

            param.min = clean_float(param.min);
            param.max = clean_float(param.max);
            param.default_value = clean_float(param.default_value);
            param.step = clean_float(param.step);

            // Prevent `step` from being 0.0
            // if param.step <= 0.0001 {
            //     // If step is missing or 0, calculate a safe step (1% of the total range)
            //     let range = param.max - param.min;
            //     let safe_step = if range > 0.0 { range / 100.0 } else { 0.01 };

            //     param.step = clean_float(safe_step);

            //     // Ultimate fallback just in case min and max were identical
            //     if param.step == 0.0 {
            //         param.step = 0.01;
            //     }
            // }
        }

        let json = serde_json::to_string_pretty(&manifest)?;
        let folder = if manifest.is_synth {
            "synths"
        } else {
            "effects"
        };

        // Use the internal_type as the file name (e.g., "karbeatzerv2.json")
        let file_name = format!("{}.manifest.json", manifest.internal_type.to_snake_case());
        let path = PathBuf::from(base_dir).join(folder).join(&file_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, json)?;

        println!(
            "✅ Exported clean manifest: {} -> {:?}",
            manifest.name, path
        );
        Ok(())
    }
}
