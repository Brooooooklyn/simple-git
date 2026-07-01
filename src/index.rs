use std::path::Path;

use napi_derive::napi;

use crate::Result;
use crate::error::IntoNapiError;

#[napi]
/// A git index (the staging area).
///
/// Obtain one with `Repository.index()`. Mutating methods change the in-memory
/// index only; call `write()` to persist it to disk, or `writeTree()` to write
/// its current state to the object database as a tree (whose OID can then be
/// used to create a commit).
pub struct Index {
  pub(crate) inner: git2::Index,
}

#[napi]
impl Index {
  #[napi]
  /// Add or update an index entry from a file on disk.
  ///
  /// The `path` is relative to the repository's working directory and must be
  /// readable. This forces the file to be added to the index even if it is
  /// ignored.
  pub fn add_path(&mut self, path: String) -> Result<()> {
    self
      .inner
      .add_path(Path::new(&path))
      .convert_without_message()
  }

  #[napi]
  /// Add or update index entries matching files in the working directory.
  ///
  /// `pathspecs` defaults to `["*"]` (everything) when omitted. Ignored files
  /// are skipped unless `force` is `true`, which maps to
  /// `IndexAddOption::FORCE`.
  pub fn add_all(&mut self, pathspecs: Option<Vec<String>>, force: Option<bool>) -> Result<()> {
    let specs = pathspecs.unwrap_or_else(|| vec!["*".to_owned()]);
    let flag = if force.unwrap_or(false) {
      git2::IndexAddOption::FORCE
    } else {
      git2::IndexAddOption::DEFAULT
    };
    self
      .inner
      .add_all(specs.iter(), flag, None)
      .convert_without_message()
  }

  #[napi]
  /// Update all index entries to match the working directory.
  ///
  /// Existing entries are refreshed and entries whose file no longer exists are
  /// removed. `pathspecs` defaults to `["*"]` when omitted. This will fail on a
  /// bare index.
  pub fn update_all(&mut self, pathspecs: Option<Vec<String>>) -> Result<()> {
    let specs = pathspecs.unwrap_or_else(|| vec!["*".to_owned()]);
    self
      .inner
      .update_all(specs.iter(), None)
      .convert_without_message()
  }

  #[napi]
  /// Remove an index entry corresponding to a file on disk.
  pub fn remove_path(&mut self, path: String) -> Result<()> {
    self
      .inner
      .remove_path(Path::new(&path))
      .convert_without_message()
  }

  #[napi]
  /// Get the count of entries currently in the index.
  pub fn size(&self) -> u32 {
    self.inner.len() as u32
  }

  #[napi]
  /// Write the in-memory index back to disk using an atomic file lock.
  pub fn write(&mut self) -> Result<()> {
    self.inner.write().convert_without_message()
  }

  #[napi]
  /// Write the index as a tree to the object database and return its OID.
  ///
  /// The index must be associated with an existing repository and must not
  /// contain any conflicted entries. The returned OID can be used to create a
  /// commit.
  pub fn write_tree(&mut self) -> Result<String> {
    self
      .inner
      .write_tree()
      .map(|oid| oid.to_string())
      .convert_without_message()
  }
}
