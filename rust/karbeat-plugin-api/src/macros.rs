#[macro_export]
macro_rules! impl_plugin_parameters {
    (
        $struct_name:ident,
        [$($field:ident),+ $(,)?]
    ) => {
        impl $struct_name {
            /// Automatically route host sets to the correct parameter
            pub fn handle_set_parameter(&mut self, id: u32, value: f32) -> bool {
                $(
                    if self.$field.id == id {
                        self.$field.set_base(value);
                        return true;
                    }
                )+
                false
            }

            /// Automatically route host gets from the correct parameter
            pub fn handle_get_parameter(&self, id: u32) -> Option<f32> {
                $(
                    if self.$field.id == id {
                        return Some(self.$field.get_base().to_f32());
                    }
                )+
                None
            }

            /// Build the defaults map dynamically
            pub fn generate_default_parameters(&self) -> std::collections::HashMap<u32, f32> {
                let mut map = std::collections::HashMap::new();
                $( map.insert(self.$field.id, self.$field.get_base().to_f32()); )+
                map
            }

            /// Build the UI specifications dynamically
            pub fn generate_parameter_specs(&self) -> Vec<karbeat_plugin_api::wrapper::PluginParameter> {
                vec![
                    $( self.$field.to_spec() ),+
                ]
            }
        }
    };
}