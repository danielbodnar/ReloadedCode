//! Opaque 64-bit hash key used for model lookup.

/// A 64-bit model key hash used by [`super::ModelCatalog`].
///
/// The concrete hash algorithm is an implementation detail and may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ModelHash(u64);

impl ModelHash {
    /// Creates a new hash wrapper.
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Returns the wrapped hash value.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}
