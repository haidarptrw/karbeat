use karbeat_core::core::project::ApplicationState;

fn main() {
    let toml_str = r#"
[metadata]
name = "Test"
author = ""
version = ""
created_at = "2026-03-31T14:00:00Z"

[mixer]
bus_counter = 1
routing = []

[mixer.master_bus]
volume = 1.0
pan = 0.0
is_muted = false
is_solo = false
effects = []
sends = []
output_bus = 0

[mixer.channels."1"]
volume = 1.0
pan = 0.0
is_muted = false
is_solo = false
effects = []
sends = []
output_bus = 1

[mixer.buses]

[transport]
bpm = 120.0
is_playing = false
is_looping = false
loop_start = 0
loop_end = 4

[transport.time_signature]
numerator = 4
denominator = 4

[asset_library]
next_id = 1

[asset_library.source_map]

[pattern_pool]
[generator_pool]
[tracks]

[automation_pool]
"1" = { id = 1, target = { ParametricEQ = { track_id = 1, effect_index = 0, band_index = 0, parameter = "Gain" } }, label = "test", min = 0.0, max = 1.0, default_value = 0.5, points = [] }

pattern_counter = 0
generator_counter = 0
track_counter = 0
automation_counter = 0
clip_counter = 0
max_sample_index = 0
"#;
    
    let app: Result<ApplicationState, _> = toml::from_str(toml_str);
    match app {
        Ok(a) => match toml::to_string(&a) {
            Ok(s) => println!("Success:\n{}", s),
            Err(e) => println!("Serialize Error: {}", e),
        },
        Err(e) => println!("Deserialize Error: {}", e),
    }
}
