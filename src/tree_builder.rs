use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use napi::bindgen_prelude::SharedReference;
use napi_derive::napi;

use crate::{ensure_alive, error::IntoNapiError};

pub(crate) enum TreeBuilderParent {
  Repository(SharedReference<crate::repo::Repository, git2::TreeBuilder<'static>>),
}

impl Deref for TreeBuilderParent {
  type Target = git2::TreeBuilder<'static>;

  fn deref(&self) -> &Self::Target {
    match self {
      TreeBuilderParent::Repository(parent) => parent.deref(),
    }
  }
}

impl DerefMut for TreeBuilderParent {
  fn deref_mut(&mut self) -> &mut Self::Target {
    match self {
      TreeBuilderParent::Repository(parent) => parent.deref_mut(),
    }
  }
}

#[napi(object)]
/// A single entry in a `TreeBuilder`, as returned by `TreeBuilder.get()`.
pub struct TreeBuilderEntry {
  /// The OID of the object the entry points to, as a 40-char hex string.
  pub oid: String,
  /// The raw git file mode of the entry (e.g. 0o100644 = 33188 for a normal
  /// blob, 0o040000 = 16384 for a tree).
  pub filemode: i32,
}

#[napi]
pub struct TreeBuilder {
  pub(crate) inner: TreeBuilderParent,
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Guards every method that derefs the underlying `git2::TreeBuilder`.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl TreeBuilder {
  pub(crate) fn inner<'repo>(&'repo self) -> &'repo git2::TreeBuilder<'repo> {
    self.inner.deref()
  }

  pub(crate) fn inner_mut(&mut self) -> &mut git2::TreeBuilder<'static> {
    self.inner.deref_mut()
  }

  #[napi]
  /// Add or update an entry in the builder.
  ///
  /// `filename` must be a single path component (this is a single-level
  /// builder, not a recursive path). `filemode` is a raw git file mode;
  /// valid values are 0o040000 (16384, tree), 0o100644 (33188, blob),
  /// 0o100755 (33261, executable blob), 0o120000 (40960, symlink) and
  /// 0o160000 (57344, submodule commit).
  ///
  /// No attempt is made to ensure that the provided OID points to an object
  /// of a reasonable type (or any object at all).
  pub fn insert(&mut self, filename: String, oid: String, filemode: i32) -> crate::Result<()> {
    ensure_alive(&self.alive)?;
    let oid = git2::Oid::from_str(&oid).convert(format!("Invalid OID [{oid}]"))?;
    self
      .inner_mut()
      .insert(filename, oid, filemode)
      .convert_without_message()
      .map(|_| ())
  }

  #[napi]
  /// Remove an entry from the builder by its filename.
  pub fn remove(&mut self, filename: String) -> crate::Result<()> {
    ensure_alive(&self.alive)?;
    self
      .inner_mut()
      .remove(filename)
      .convert_without_message()
  }

  #[napi]
  /// Get an entry from the builder from its filename.
  ///
  /// Returns `null` when no entry with that filename exists.
  pub fn get(&self, filename: String) -> crate::Result<Option<TreeBuilderEntry>> {
    ensure_alive(&self.alive)?;
    self
      .inner()
      .get(filename)
      .map(|entry| {
        entry.map(|entry| TreeBuilderEntry {
          oid: entry.id().to_string(),
          filemode: entry.filemode(),
        })
      })
      .convert_without_message()
  }

  #[napi]
  /// Write the contents of the builder as a tree object into the
  /// repository's object database and return its OID hex string.
  pub fn write(&self) -> crate::Result<String> {
    ensure_alive(&self.alive)?;
    self
      .inner()
      .write()
      .map(|oid| oid.to_string())
      .convert_without_message()
  }

  #[napi]
  /// Clear all the entries in the builder.
  pub fn clear(&mut self) -> crate::Result<()> {
    ensure_alive(&self.alive)?;
    self.inner_mut().clear().convert_without_message()
  }

  #[napi]
  /// Get the number of entries listed in the builder.
  pub fn len(&self) -> crate::Result<u32> {
    ensure_alive(&self.alive)?;
    Ok(self.inner().len() as u32)
  }

  #[napi]
  /// Return `true` if there is no entry in the builder.
  pub fn is_empty(&self) -> crate::Result<bool> {
    ensure_alive(&self.alive)?;
    Ok(self.inner().is_empty())
  }
}
