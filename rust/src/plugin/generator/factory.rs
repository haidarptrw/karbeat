// src/plugin/generator/factory.rs

use crate::{core::plugin::KarbeatPlugin, plugin::generator::karbeatzer::Karbeatzer};

pub struct KarbeatGeneratorFactory {}

impl KarbeatGeneratorFactory {
    pub fn create(plugin_name: &str, sample_rate: Option<f32>) -> anyhow::Result<KarbeatPlugin> {
        match plugin_name {
            "Karbeatzer" => {
                let karbeatzer = KarbeatGeneratorFactory::karbeatzer(sample_rate);
                Ok(KarbeatPlugin::Generator(Box::new(karbeatzer)))
            }
            _ => {
                Err(anyhow::anyhow!("Plugin unavailable"))
            }

        }
    }
    pub fn karbeatzer(sample_rate: Option<f32> ) -> Karbeatzer {
        Karbeatzer::new(sample_rate)
    }
}