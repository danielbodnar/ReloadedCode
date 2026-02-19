//! Fixed4 wrapper for model temperature values.

use super::sampling_fixed4::Fixed4;

/// Temperature encoded in fixed4 (`10_000 => 1.0000`).
///
/// The encoded value `65_535` is reserved as a `None` sentinel in packed
/// optional fields and is therefore not a valid `TemperatureFixed4` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TemperatureFixed4(Fixed4);

impl TemperatureFixed4 {
    /// Maximum encoded value representable by [`TemperatureFixed4`].
    pub const MAX_ENCODED: u16 = Fixed4::MAX_ENCODED;

    /// Creates from an encoded fixed4 value.
    ///
    /// Returns `None` when the input equals the reserved `None` sentinel.
    #[inline]
    pub const fn from_encoded(encoded: u16) -> Option<Self> {
        match Fixed4::from_encoded(encoded) {
            Some(fixed4) => Some(Self(fixed4)),
            None => None,
        }
    }

    /// Returns the encoded fixed4 representation.
    #[inline]
    pub const fn encoded(self) -> u16 {
        self.0.encoded()
    }

    /// Returns the user-facing floating-point value.
    #[inline]
    pub fn value(self) -> f64 {
        self.0.value()
    }

    #[inline]
    pub(crate) const fn encode_optional(value: Option<Self>) -> u16 {
        match value {
            Some(t) => t.0.encoded(),
            None => Fixed4::NONE_SENTINEL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_is_not_accepted() {
        assert!(TemperatureFixed4::from_encoded(Fixed4::NONE_SENTINEL).is_none());
    }

    #[test]
    fn value_roundtrips() {
        let value = TemperatureFixed4::from_encoded(12_000).expect("valid fixed4 value");
        assert_eq!(value.encoded(), 12_000);
        assert_eq!(value.value(), 1.2);
    }

    #[test]
    fn max_encoded_excludes_sentinel() {
        assert_eq!(TemperatureFixed4::MAX_ENCODED, Fixed4::NONE_SENTINEL - 1);
    }
}
