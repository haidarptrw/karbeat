#[cfg(target_os="android")] 
use once_cell::sync::OnceCell;

#[cfg(target_os="android")]
use jni::{objects::JObject, refs::Global};

use crate::{init_engine, init_logger};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    init_logger();
    init_engine();
    log::info!("DAW Engine System Started via FRB Init");
}

// ============================================================================
// ANDROID CONTEXT INITIALIZATION (JNI)
// ============================================================================

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: jni::JavaVM, _res: *mut std::ffi::c_void) -> jni::sys::jint {
    log::info!("JNI_OnLoad Triggered in Rust");

    // Upgrade the JavaVM pointer to a safe JNIEnv using the closure API
    let _outcome = vm.with_top_local_frame(|env| -> Result<(), jni::errors::Error> {
        
        // You can safely use the `env` inside this block.
        // For example, this is the perfect place to do `env.find_class()` 
        // lookups once and cache the GlobalRefs for later use.

        Ok(())
    });

    // If you actually needed to handle the error when getting the env, 
    // you could do: `.expect("Cannot get reference to the JNIEnv");` on _outcome.

    jni::sys::JNI_VERSION_1_6
}

// We need a safe, static place to store the Global Reference so it is never dropped.
#[cfg(target_os="android")]
static ANDROID_CONTEXT: OnceCell<Global<JObject<'static>>> = OnceCell::new();

#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_karbeat_MainActivity_initRust(
    mut unowned_env: jni::EnvUnowned,
    _class: jni::objects::JClass,
    context: jni::objects::JObject,
) {
    log::info!("Initializing ndk-context from Kotlin...");
    
   let _outcome = unowned_env.with_env(|env| -> Result<(), jni::errors::Error> {
        
        let vm = env.get_java_vm()?;

        // Upgrade the temporary Local Reference to a permanent Global Reference
        let global_context = env.new_global_ref(&context)?;
        
        // Store it statically so it lives forever (ignoring errors if it's already set)
        let _ = ANDROID_CONTEXT.set(global_context);

        // Extract the raw pointer from our safely stored Global Reference
        let global_context_raw = ANDROID_CONTEXT.get().unwrap().as_raw();

        // Pass the global pointer to ndk-context
        ndk_context::initialize_android_context(
            vm.get_raw() as *mut std::ffi::c_void,
            global_context_raw as *mut std::ffi::c_void,
        );

        Ok(())
    });
    
    log::info!("ndk-context initialized successfully!");
}