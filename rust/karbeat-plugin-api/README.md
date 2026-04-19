# Karbeat Plugin API

This is the documentation for the implementation of Karbeat's Plugin API. We will
talk about what you need to know and how to implement the API

(for more detail on this will be added in the future). To see more examples you can see it [here](../karbeat-plugins/)

## Scope

### 1. Base Trait

The core definition for both generator/synth and effect can be found at [traits.rs](./src/traits.rs). This file
includes the trait user needs to implement

```rs
pub trait KarbeatEffect: Send + Sync {
    fn name(&self) -> &str;
    ...
}

pub trait KarbeatGenerator: Send + Sync  {
    fn name(&self) -> &str;
    ...
}
```

User can freely decided how to implement required method freely. 
Though we also provide a wrapper that can be used to add base parameter specifications, 
**User can implement the mapping of each parameter freely**, but with a guarantee that the frontend knows the mapping of each parameter. For it to be less error-prone. you can the map parameter integer keys to
an enum or constant which both the frontend and the backend agrees.

### 2. Base Synth & Base Effect

We provided reserved parameters for common DSP found in synths or effects,
and we put it as base implementation so we don't have to rewrite this part
each time you want to implement some plugin. So, you can focus on implementing
the complex audio signal processing for the plugin.

For more detail see [effect_base.rs](./src/effect_base.rs) and [synth_base.rs](./src/synth_base.rs)

### 3. Plugin Wrapper

Plugin wrapper in [wrapper.rs](./src/wrapper.rs) lets you easily integrate
your plugin logic with the traits and plugin base setup so you don't have to worry about
wiring your implemented logic with the API

**Example of implementation.**

To see examples of implementation you can see it [here](../karbeat-plugins/)

### 4. Macro

Implementing this with all the boilerplate might be tedious. So, we have prepared an macro
to help implementing the macro (see more at [karbeat_macros](../karbeat-macros/src/lib.rs))

Example:

```rust
#[derive(Clone)]
#[karbeat_plugin]
pub struct KarbeatzerEngine {
    // IMPORTANT: Oscillator needs to implement AutoParams to make this work. 
    // it depends on `karbeat-plugin-types` crate, so you have to include it in the Cargo.toml
    #[nested]
    oscillators: [Oscillator; 3], 
    #[param(id = 8, name = "Drive", group = "Master", min = 0.0, max = 1.0, default = 0.0)]
    drive: f32,
}

// This will generate the auto implementation for boilerplate parameter manipulation.
// you can just call it like this
impl RawSynthEngine for KarbeatzerEngine {
    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        self.auto_set_parameter(id, value);
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        self.auto_get_parameter(id)
    }

    fn apply_automation(&mut self, id: u32, value: f32) {
        self.auto_apply_automation(id, value);
    }

    fn clear_automation(&mut self, id: u32) {
        self.auto_clear_automation(id);
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        self.auto_get_parameter_specs()
    }
}
```

### 5. Adding to Registry

To add it to registry, the inner struct must implement Default so that the registry
can call the builder

```rust
impl Default for PluginEngine {
    fn default() -> Self {
        // ...Your default implementation
    }
}

// This will generate YourAwesomePlugin::build() Automatically
pub type YourAwesomePlugin = RawSynthWrapper<PluginEngine>;

```

## ⚠ Limitations and Important Note

- **Currently, there are only limited methods can be implemented in the trait.
As the project grows, more trait are added, and may cause breaking changes.
As the current state of the development is still on the alpha phase, you will expect this very often**

- Currently the Base Wrapper is still unstable and may cause a lot of bug. We recommended to build the DSP from scratch
using provided building blocks inside the `karbeat-dsp` crate


