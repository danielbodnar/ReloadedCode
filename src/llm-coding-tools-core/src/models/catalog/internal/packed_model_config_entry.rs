//! Packed optional model sampling configuration entry.
//!
//! Layout (`u32`):
//! - `16` bits: temperature fixed4 (with `u16::MAX` as `None` sentinel)
//! - `16` bits: top_p fixed4 (with `u16::MAX` as `None` sentinel)

use super::{Fixed4, TemperatureFixed4, TopPFixed4};
use crate::models::catalog::public::builder_types::ModelConfig;
use bitfields::bitfield;

/// Packed model-configuration sidecar row.
#[bitfield(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackedModelConfigEntry {
    temperature: u16,
    top_p: u16,
}

impl PackedModelConfigEntry {
    /// Creates a packed row from optional public model config.
    #[inline]
    pub fn from_model_config(config: Option<ModelConfig>) -> Self {
        let mut packed = Self::new_without_defaults();
        match config {
            Some(config) => {
                packed.set_temperature(TemperatureFixed4::encode_optional(config.temperature));
                packed.set_top_p(TopPFixed4::encode_optional(config.top_p));
            }
            None => {
                packed.set_temperature(Fixed4::NONE_SENTINEL);
                packed.set_top_p(Fixed4::NONE_SENTINEL);
            }
        }
        packed
    }

    /// Returns true when both fields are the `None` sentinel.
    #[inline]
    pub const fn is_none(self) -> bool {
        Fixed4::is_sentinel(self.temperature()) && Fixed4::is_sentinel(self.top_p())
    }

    /// Converts a packed row into optional public model config.
    #[inline]
    pub fn into_model_config(self) -> Option<ModelConfig> {
        let temperature = TemperatureFixed4::from_encoded(self.temperature());
        let top_p = TopPFixed4::from_encoded(self.top_p());
        if temperature.is_none() && top_p.is_none() {
            None
        } else {
            Some(ModelConfig { temperature, top_p })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_model_config_entry_is_4_bytes() {
        assert_eq!(core::mem::size_of::<PackedModelConfigEntry>(), 4);
    }

    #[test]
    fn none_roundtrips() {
        let packed = PackedModelConfigEntry::from_model_config(None);
        assert!(packed.is_none());
        assert_eq!(packed.into_model_config(), None);
    }

    #[test]
    fn values_roundtrip() {
        let packed = PackedModelConfigEntry::from_model_config(Some(ModelConfig {
            temperature: TemperatureFixed4::from_encoded(12_000),
            top_p: TopPFixed4::from_encoded(5_000),
        }));

        let unpacked = packed.into_model_config().expect("config must exist");
        assert_eq!(
            unpacked
                .temperature
                .expect("temperature must exist")
                .encoded(),
            12_000
        );
        assert_eq!(unpacked.top_p.expect("top_p must exist").encoded(), 5_000);
    }
}
