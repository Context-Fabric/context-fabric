//! Atomic replacement of files that other processes may be reading.

use std::fs::File;
use std::path::Path;

use crate::error::{CfError, Result};

/// Writes `path` by staging the content in a temporary file in the same
/// directory and renaming it over `path` once `write` succeeds.
///
/// Readers therefore see either the previous file or the complete new one,
/// never a partial write; readers that memory-mapped the previous file keep
/// its bytes; and a failed `write` leaves the previous file untouched. The
/// staged file gets the permissions `File::create` would (0o666 minus the
/// umask on Unix) rather than tempfile's private 0o600, so files in shared
/// corpus directories stay readable by other users.
pub(crate) fn write_atomically(path: &Path, write: impl FnOnce(&File) -> Result<()>) -> Result<()> {
    let io_error = |source| CfError::Io {
        path: path.to_path_buf(),
        source,
    };
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let prefix = format!(
        ".{}-",
        path.file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default()
    );
    let mut builder = tempfile::Builder::new();
    builder.prefix(&prefix).suffix(".partial");
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
    let staged = builder.tempfile_in(directory).map_err(io_error)?;
    write(staged.as_file())?;
    staged
        .persist(path)
        .map_err(|error| io_error(error.error))?;
    Ok(())
}
