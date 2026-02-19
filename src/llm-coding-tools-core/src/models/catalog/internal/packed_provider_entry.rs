//! Packed provider metadata entry.
//!
//! Layout (`u8`):
//! - `8` bits: [`crate::models::ProviderType`] discriminant

use crate::models::ProviderType;
use bitfields::bitfield;

/// Packed provider metadata row.
#[bitfield(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackedProviderEntry {
    provider_type: u8,
}

impl PackedProviderEntry {
    /// Creates one packed provider metadata row.
    #[inline]
    pub fn from_parts(provider_type: ProviderType) -> Self {
        let mut packed = Self::new_without_defaults();
        packed.set_provider_type(provider_type.to_u8());
        packed
    }

    /// Returns decoded provider type.
    #[inline]
    pub fn api_type(self) -> ProviderType {
        ProviderType::from_u8(self.provider_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_provider_entry_is_1_byte() {
        assert_eq!(core::mem::size_of::<PackedProviderEntry>(), 1);
    }

    #[test]
    fn provider_entry_roundtrip() {
        let packed = PackedProviderEntry::from_parts(ProviderType::Azure);
        assert_eq!(packed.api_type(), ProviderType::Azure);
    }
}
