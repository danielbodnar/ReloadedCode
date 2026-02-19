//! Fixed4 decimal type for sampling values.
//!
//! Fixed4 means values are stored as `value * 10_000` in a `u16`.
//! For example, `1.0` is encoded as `10_000`.

/// Fixed-point decimal with 4 fractional digits.
///
/// Encoded as `value * 10_000` in a `u16`. The value `u16::MAX` is reserved
/// as a `None` sentinel for packed optional fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Fixed4(u16);

impl Fixed4 {
    /// Scale factor (`10_000 => 1.0000`).
    pub const SCALE: u16 = 10_000;
    /// Sentinel representing `None` in packed optional fields.
    pub const NONE_SENTINEL: u16 = u16::MAX;
    /// Maximum encodable value (excludes the `None` sentinel).
    pub const MAX_ENCODED: u16 = u16::MAX - 1;

    /// Creates from an encoded fixed4 value.
    ///
    /// Returns `None` when the input equals [`Self::NONE_SENTINEL`].
    #[inline]
    pub const fn from_encoded(encoded: u16) -> Option<Self> {
        if encoded == Self::NONE_SENTINEL {
            None
        } else {
            Some(Self(encoded))
        }
    }

    /// Returns the encoded fixed4 representation.
    #[inline]
    pub const fn encoded(self) -> u16 {
        self.0
    }

    /// Returns the user-facing floating-point value.
    #[inline]
    pub fn value(self) -> f64 {
        f64::from(self.0) / f64::from(Self::SCALE)
    }

    /// Encodes an optional `Fixed4` for packed storage.
    ///
    /// Returns [`Self::NONE_SENTINEL`] when `opt` is `None`.
    #[inline]
    #[cfg(test)]
    pub const fn encode_optional(opt: Option<Self>) -> u16 {
        match opt {
            Some(fixed4) => fixed4.0,
            None => Self::NONE_SENTINEL,
        }
    }

    /// Returns `true` if the raw encoded value is the `None` sentinel.
    #[inline]
    pub const fn is_sentinel(encoded: u16) -> bool {
        encoded == Self::NONE_SENTINEL
    }

    /// Returns `true` if the raw encoded value represents a valid `Fixed4`.
    #[inline]
    #[cfg(test)]
    #[allow(dead_code)] // public api
    pub const fn is_valid(encoded: u16) -> bool {
        encoded != Self::NONE_SENTINEL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_returns_none() {
        assert!(Fixed4::from_encoded(Fixed4::NONE_SENTINEL).is_none());
    }

    #[test]
    fn value_roundtrips() {
        let fixed4 = Fixed4::from_encoded(12_000).expect("valid fixed4 value");
        assert_eq!(fixed4.encoded(), 12_000);
        assert_eq!(fixed4.value(), 1.2);
    }

    #[test]
    fn optional_encoding_roundtrips() {
        let some_value = Fixed4::from_encoded(5_000).expect("valid fixed4 value");
        let encoded = Fixed4::encode_optional(Some(some_value));
        assert_eq!(encoded, 5_000);
        assert_eq!(Fixed4::from_encoded(encoded), Some(some_value));

        let none_encoded = Fixed4::encode_optional(None);
        assert_eq!(none_encoded, Fixed4::NONE_SENTINEL);
        assert!(Fixed4::from_encoded(none_encoded).is_none());
    }

    #[test]
    fn max_encoded_excludes_sentinel() {
        assert_eq!(Fixed4::MAX_ENCODED, Fixed4::NONE_SENTINEL - 1);
    }
}
