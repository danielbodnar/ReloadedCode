//! Tokio-based async filesystem operations.

use std::io::ErrorKind;
use std::path::Path;
use tokio::io::AsyncReadExt as _;

/// Reads a file into memory in one pre-sized allocation.
#[inline]
pub(crate) async fn read(path: impl AsRef<Path>) -> std::io::Result<Box<[u8]>> {
    let mut file = tokio::fs::File::open(path).await?;
    let file_len_u64 = file.metadata().await?.len();
    let file_len = usize::try_from(file_len_u64).map_err(|_| {
        std::io::Error::new(ErrorKind::InvalidData, "file is too large to fit in memory")
    })?;

    let mut bytes = super::alloc_uninit_u8_slice(file_len);
    if file_len != 0 {
        let buf = super::uninit_u8_slice_as_mut_bytes(&mut bytes);
        file.read_exact(buf).await?;
    }

    Ok(super::assume_init_u8_slice(bytes))
}

/// Writes all bytes to a file, creating or truncating it.
#[inline]
pub(crate) async fn write(path: impl AsRef<Path>, bytes: &[u8]) -> std::io::Result<()> {
    tokio::fs::write(path, bytes).await
}

/// Creates a directory and all parent directories.
#[inline]
pub(crate) async fn create_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    tokio::fs::create_dir_all(path).await
}

/// Renames a file, replacing the destination if it exists.
#[inline]
pub(crate) async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
    tokio::fs::rename(from, to).await
}
