//! Atomic publication of files that other processes may be reading.
//!
//! A [`StagedFile`] is written under a temporary name in the destination's
//! directory and only then renamed to its final name, so readers see either
//! no file, the previous file, or the complete new one, never a partial
//! write. A staged file that is dropped before it is published (because
//! writing failed, or another writer published first) is deleted.

use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use tempfile::{NamedTempFile, PersistError};

use crate::error::{CfError, Result};

/// Suffix of the temporary name a [`StagedFile`] is written under.
pub(crate) const STAGED_SUFFIX: &str = ".partial";

/// A file being written under a temporary name beside its final `path`.
pub(crate) struct StagedFile {
    temp: NamedTempFile,
    path: PathBuf,
}

/// Outcome of [`StagedFile::publish_new`].
pub(crate) enum Publication {
    /// The staged file now has its final name.
    Published,
    /// Another file already has the final name; the staged file is handed
    /// back unpublished (dropping it deletes it).
    Occupied(StagedFile),
}

impl StagedFile {
    /// Creates an empty staged file for `path` in the same directory.
    ///
    /// The staged file gets the permissions `File::create` would (0o666 minus
    /// the umask on Unix) rather than tempfile's private 0o600, so files in
    /// shared corpus directories stay readable by other users.
    pub(crate) fn new(path: &Path) -> Result<Self> {
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
        builder.prefix(&prefix).suffix(STAGED_SUFFIX);
        #[cfg(unix)]
        builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
        let temp = builder
            .tempfile_in(directory)
            .map_err(|source| io_error(path, source))?;
        Ok(Self {
            temp,
            path: path.to_path_buf(),
        })
    }

    /// The staged file, open for writing.
    pub(crate) fn file(&self) -> &File {
        self.temp.as_file()
    }

    /// Gives the staged file its final name unless a file already has it.
    ///
    /// Never replacing an existing file works the same on every platform,
    /// including Windows, where a file another process has open or
    /// memory-mapped cannot be replaced.
    pub(crate) fn publish_new(self) -> Result<Publication> {
        let path = self.path;
        match self.temp.persist_noclobber(&path) {
            Ok(_) => Ok(Publication::Published),
            Err(PersistError { error, file }) if error.kind() == ErrorKind::AlreadyExists => {
                Ok(Publication::Occupied(Self { temp: file, path }))
            }
            Err(PersistError { error, .. }) => Err(io_error(&path, error)),
        }
    }

    /// Gives the staged file its final name, replacing any file that has it.
    ///
    /// Readers that memory-mapped the replaced file keep its bytes on Unix.
    /// Windows refuses to replace a file that is open or mapped; that fails
    /// with [`ErrorKind::PermissionDenied`].
    pub(crate) fn publish_replacing(self) -> Result<()> {
        let path = self.path;
        self.temp
            .persist(&path)
            .map(drop)
            .map_err(|PersistError { error, .. }| io_error(&path, error))
    }
}

/// Writes `path` through a [`StagedFile`] that replaces any previous file
/// once `write` succeeds; a failed `write` leaves the previous file untouched.
pub(crate) fn write_atomically(path: &Path, write: impl FnOnce(&File) -> Result<()>) -> Result<()> {
    let staged = StagedFile::new(path)?;
    write(staged.file())?;
    staged.publish_replacing()
}

fn io_error(path: &Path, source: io::Error) -> CfError {
    CfError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use super::*;

    fn staged_names(directory: &Path) -> Vec<String> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(STAGED_SUFFIX))
            .collect()
    }

    fn stage(path: &Path, content: &[u8]) -> StagedFile {
        let staged = StagedFile::new(path).unwrap();
        staged.file().write_all(content).unwrap();
        staged
    }

    #[test]
    fn publish_new_names_the_staged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.bin");
        assert!(matches!(
            stage(&path, b"new").publish_new().unwrap(),
            Publication::Published
        ));
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(staged_names(dir.path()).is_empty());
    }

    #[test]
    fn publish_new_never_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.bin");
        fs::write(&path, b"first").unwrap();
        let Publication::Occupied(staged) = stage(&path, b"second").publish_new().unwrap() else {
            panic!("an existing file was replaced");
        };
        assert_eq!(staged_names(dir.path()).len(), 1);
        drop(staged);
        assert_eq!(fs::read(&path).unwrap(), b"first");
        assert!(staged_names(dir.path()).is_empty());
    }

    #[test]
    fn an_occupied_publication_can_still_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.bin");
        fs::write(&path, b"first").unwrap();
        let Publication::Occupied(staged) = stage(&path, b"second").publish_new().unwrap() else {
            panic!("an existing file was replaced");
        };
        staged.publish_replacing().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(staged_names(dir.path()).is_empty());
    }

    #[test]
    fn a_failed_write_leaves_the_previous_file_and_no_staged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.bin");
        fs::write(&path, b"previous").unwrap();
        let result = write_atomically(&path, |mut file| {
            file.write_all(b"partial")?;
            Err(CfError::MissingFeature("otype".to_string()))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        assert!(staged_names(dir.path()).is_empty());
    }
}
