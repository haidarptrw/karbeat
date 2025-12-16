use crate::{init_engine, init_logger};

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    init_logger();

    init_engine(); 
    
    // 3. Log it
    log::info!("DAW Engine System Started via FRB Init");
}
