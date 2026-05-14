use karbeat_plugin_api::manifest::Manifestable;
use karbeat_plugins::plugins::*;

#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
fn main() {
    println!("Starting Karbeat Manifest Extractor...");

    // Points to shared assets folder.
    // This runs from the root of the rust workspace.
    let export_dir = std::env::var("PLUGIN_MANIFEST_DIR")
        .unwrap_or("../../assets/manifests/audio-plugins".into());

    // Call the export function on all plugins
    MyRetro::export_manifest(&export_dir).unwrap();
    KarbeatzerV2::export_manifest(&export_dir).unwrap();
    DigiParametricEQ::export_manifest(&export_dir).unwrap();

    println!("INFO: All manifests exported successfully to Flutter assets!");
}
