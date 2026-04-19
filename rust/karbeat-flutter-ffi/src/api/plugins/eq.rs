use flutter_rust_bridge::frb;
use karbeat_core::{core::project::{TrackId}, lock::{get_app_read, get_plugin_registry_read}};
use karbeat_core::shared::id::*;
use karbeat_plugins::effect::parametric_eq::KarbeatParametricEQ;

use crate::api::plugin::UiEffectTarget;

/// A point on the EQ response curve (DTO for FRB)
#[derive(Clone, Debug)]
pub struct UiResponseCurvePoint {
    pub frequency: f32,
    pub magnitude_db: f32,
}

/// Compute the magnitude response curve for a parametric EQ effect on a track.
///
/// Creates a temporary plugin instance, applies stored parameters, and evaluates
/// the exact biquad transfer function at log-spaced frequency points.
pub fn get_eq_response_curve(
    target: UiEffectTarget,
    effect_id: u32,
    num_points: u32
) -> Result<Vec<UiResponseCurvePoint>, String> {
    let app = get_app_read();
    let effect_id_typed = EffectId::from(effect_id);

    let (plugin_name, plugin_registry_id, plugin_parameters) = match target {
        UiEffectTarget::Track(track_id) => {
            let channel = app.mixer.channels
                .get(&TrackId::from(track_id))
                .ok_or_else(|| format!("Track channel {} not found", track_id))?;
            let effect = channel.effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Bus(bus_id) => {
            let bus = app.mixer.buses
                .get(&BusId::from(bus_id))
                .ok_or_else(|| format!("Bus {} not found", bus_id))?;
            let effect = bus.channel.effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
        UiEffectTarget::Master => {
            let effect = app.mixer.master_bus.effects
                .iter()
                .find(|e| e.id == effect_id_typed)
                .ok_or_else(|| format!("Effect {} not found", effect_id))?;
            (
                effect.instance.name.clone(),
                effect.instance.registry_id,
                effect.instance.parameters.clone(),
            )
        }
    };

    // Create a temporary plugin from registry
    let registry = get_plugin_registry_read();
    let temp_plugin = if plugin_registry_id > 0 {
        registry.create_effect_by_id(plugin_registry_id).map(|(plugin, _)| plugin)
    } else {
        registry.create_effect(&plugin_name)
    };

    let mut temp_plugin = temp_plugin.ok_or_else(||
        format!("Effect '{}' not found in registry", plugin_name)
    )?;

    // Apply stored parameters to the temp plugin
    for (&param_id, &value) in &plugin_parameters {
        temp_plugin.set_parameter(param_id, value);
    }

    // Downcast to KarbeatParametricEQ to access compute_magnitude_response
    let eq = temp_plugin
        .as_any()
        .downcast_ref::<KarbeatParametricEQ>()
        .ok_or_else(|| "Effect is not a Parametric EQ".to_string())?;

    let response = eq.engine.compute_magnitude_response(num_points as usize);

    Ok(
        response
            .into_iter()
            .map(|(freq, mag_db)| UiResponseCurvePoint {
                frequency: freq,
                magnitude_db: mag_db,
            })
            .collect()
    )
}

#[frb(sync)]
pub fn parse_eq_curve_response(json_str: String) -> Result<Vec<UiResponseCurvePoint>, String> {
    // Parse the string into a generic JSON Value
    let payload: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Ensure it's an array
    let array = payload.as_array()
        .ok_or_else(|| "Expected a JSON array from EQ response".to_string())?;

    // Map it to the strongly-typed Rust struct
    let points: Vec<UiResponseCurvePoint> = array.iter().filter_map(|val| {
        let freq = val.get("frequency")?.as_f64()? as f32;
        let mag = val.get("magnitude_db")?.as_f64()? as f32;
        Some(UiResponseCurvePoint {
            frequency: freq,
            magnitude_db: mag,
        })
    }).collect();

    Ok(points)
}